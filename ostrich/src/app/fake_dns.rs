use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use crate::app::dispatcher::Dispatcher;
use crate::proxy::UdpOutboundHandler;
use crate::session::{Session, SocksAddr};
use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ByteOrder};
use log::*;
use tokio::io::AsyncBufReadExt;
use trust_dns_proto::op::{
    header::MessageType, op_code::OpCode, response_code::ResponseCode, Message,
};
use trust_dns_proto::rr::{
    dns_class::DNSClass, record_data::RData, record_type::RecordType, resource::Record,
};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime};
use trust_dns_resolver::{AsyncResolver, Resolver};

pub enum FakeDnsMode {
    Include,
    Exclude,
}

pub struct FakeDns {
    ip_to_domain: HashMap<u32, String>,
    domain_to_ip: HashMap<String, u32>,
    cursor: u32,
    min_cursor: u32,
    max_cursor: u32,
    ttl: u32,
    filters: Vec<String>,
    mode: FakeDnsMode,
}

impl FakeDns {
    pub fn new(mode: FakeDnsMode) -> Self {
        let min_cursor = Self::ip_to_u32(&Ipv4Addr::new(198, 18, 0, 0));
        let max_cursor = Self::ip_to_u32(&Ipv4Addr::new(198, 18, 4, 255));

        let options = ResolverOpts::default();
        let nameservers: Vec<SocketAddr> = vec![
            // Cloudflare
            "108.61.199.26:2053",
            // "1.0.0.1:53"
        ]
        .iter()
        .map(|server| {
            server
                .parse::<SocketAddr>()
                .expect(format!("can not parse ip:{}", &server).as_str())
        })
        .collect();

        FakeDns {
            ip_to_domain: HashMap::new(),
            domain_to_ip: HashMap::new(),
            cursor: min_cursor,
            min_cursor,
            max_cursor,
            ttl: 1,
            filters: Vec::new(),
            mode,
        }
    }

    pub fn add_filter(&mut self, filter: String) {
        self.filters.push(filter);
    }

    fn allocate_ip(&mut self, domain: &str) -> Ipv4Addr {
        if let Some(prev_domain) = self.ip_to_domain.insert(self.cursor, domain.to_owned()) {
            // Remove the entry in the reverse map to make sure we won't have
            // multiple domains point to a same IP.
            self.domain_to_ip.remove(&prev_domain);
        }
        self.domain_to_ip.insert(domain.to_owned(), self.cursor);
        let ip = Self::u32_to_ip(self.cursor);
        self.cursor += 1;
        if self.cursor > self.max_cursor {
            self.cursor = self.min_cursor;
        }
        ip
    }

    pub fn query_domain(&mut self, ip: &IpAddr) -> Option<String> {
        let ip = match ip {
            IpAddr::V4(ip) => ip,
            _ => return None,
        };
        self.ip_to_domain.get(&Self::ip_to_u32(ip)).cloned()
    }

    pub fn query_fake_ip(&mut self, domain: &str) -> Option<IpAddr> {
        self.domain_to_ip
            .get(domain)
            .map(|v| IpAddr::V4(Self::u32_to_ip(v.to_owned())))
    }

    fn accept(&self, domain: &str) -> bool {
        match self.mode {
            FakeDnsMode::Exclude => {
                for d in &self.filters {
                    if domain.contains(d) || d == "*" {
                        return false;
                    }
                }
                true
            }
            FakeDnsMode::Include => {
                for d in &self.filters {
                    if domain.contains(d) || d == "*" {
                        return true;
                    }
                }
                false
            }
        }
    }

    pub async fn generate_fake_response(
        &mut self,
        request: &[u8],
        dispatcher: Arc<Dispatcher>,
    ) -> Result<Vec<u8>> {
        use trust_dns_proto::{
            op::{header::MessageType, op_code::OpCode, query::Query, Message},
            rr::{record_type::RecordType, Name},
        };

        let req = Message::from_vec(request)?;

        if req.queries().is_empty() {
            return Err(anyhow!("no queries in this DNS request"));
        }

        let query = &req.queries()[0];
        if query.query_class() != DNSClass::IN {
            return Err(anyhow!("unsupported query class {}", query.query_class()));
        }

        let t = query.query_type();
        if t != RecordType::A && t != RecordType::AAAA && t != RecordType::HTTPS {
            return Err(anyhow!(
                "unsupported query record type {:?}",
                query.query_type()
            ));
        }

        let raw_name = query.name();

        // TODO check if a valid domain
        let domain = if raw_name.is_fqdn() {
            let fqdn = raw_name.to_ascii();
            fqdn[..fqdn.len() - 1].to_string()
        } else {
            raw_name.to_ascii()
        };

        if !self.accept(&domain) {
            return Err(anyhow!("domain {} not accepted", domain));
        }

        /*        let handler = if let Some(v) = dispatcher.outbound_manager.read().await.get("tag") {
            v
        } else {
            println!("outbound {} not found", tag);
            return  Err(anyhow!("outbound {} not found", tag));;
        };
        println!("testing outbound {}", &handler.tag());*/

        let start = tokio::time::Instant::now();
        let sess = Session {
            destination: SocksAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53)),
            ..Default::default()
        };

        // new socket to communicate with the target.
        let socket = match dispatcher.dispatch_udp(&sess).await {
            Ok(s) => s,
            Err(e) => {
                // sessions.lock().await.remove(&raddr);
                return Err(anyhow!("dispatch udp error {:?}", e));
            }
        };

        let addr = SocksAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53));

        let (mut recv, mut send) = socket.split();
        if let Err(e) = send.send_to(&request, &addr).await {
            debug!("send message failed: {}", e);
        }
        let mut buf = [0u8; 1500];
        match recv.recv_from(&mut buf).await {
            Ok((i, _)) => {
                let resp = &buf[..i];
                let elapsed = tokio::time::Instant::now().duration_since(start);
                debug!(
                    "received response from outbound in {}ms",
                    elapsed.as_millis()
                );
                debug!("dns parse message {:?}", Message::from_vec(resp)?);
                Ok(resp.to_vec())
            }
            Err(e) => {
                debug!("receive from outbound  failed: {}", e);
                Err(anyhow::anyhow!("receive from outbound failed: {}", e))
            }
        }

        /*        match crate::proxy::connect_udp_outbound(&sess, dns_client, &handler).await {
            Ok(transport) => {
                match UdpOutboundHandler::handle(handler.as_ref(), &sess, transport).await {
                    Ok(socket) => {
                        let addr =
                            SocksAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53));
                        let mut msg = Message::new();
                        let name = Name::from_str("www.google.com.").unwrap();
                        let query = Query::query(name, RecordType::A);
                        msg.add_query(query);
                        // let mut rng = StdRng::from_entropy();
                        // let id: u16 = rng.gen();
                        msg.set_id(req.id());
                        msg.set_op_code(OpCode::Query);
                        msg.set_message_type(MessageType::Query);
                        msg.set_recursion_desired(true);
                        let msg_buf = msg.to_vec().unwrap();
                        let (mut recv, mut send) = socket.split();
                        if let Err(e) = send.send_to(&msg_buf, &addr).await {
                            println!("send message to {} failed: {}", &handler.tag(), e);
                        }
                        let mut buf = [0u8; 1500];
                        match recv.recv_from(&mut buf).await {
                            Ok(_) => {
                                let elapsed = tokio::time::Instant::now().duration_since(start);
                                println!(
                                    "received response from outbound {} in {}ms",
                                    &handler.tag(),
                                    elapsed.as_millis()
                                );
                            }
                            Err(e) => {
                                println!("receive from outbound {} failed: {}", &handler.tag(), e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("dispatch to outbound {} failed: {}", &handler.tag(), e);
                    }
                }
            }
            Err(e) => {
                println!("dispatch to outbound {} failed: {}", &handler.tag(), e);
            }
        };*/

        // let lookup = self.trustable_resolver.lookup_ip(domain.clone() + ".").await?;
        // let ip = if let Some(ip) = self.query_fake_ip(&domain) {
        //     match ip {
        //         IpAddr::V4(a) => a,
        //         _ => return Err(anyhow!("unexpected Ipv6 fake IP")),
        //     }
        // } else {
        //     let ip = self.allocate_ip(&domain);
        //     debug!("allocate {} for {}", &ip, &domain);
        //     ip
        // };

        /*        let record = lookup.as_lookup().record_iter().next().unwrap();
                let record_type = record.record_type();
                let ttl = record.ttl();
                let class = record.dns_class();
                let name = record.name();
                let ip =  match lookup.iter().next().unwrap(){
                    IpAddr::V4(a) => {
                        RData::A(a)
                    }
                    IpAddr::V6(aaa) => {
                        RData::AAAA(aaa)
                    }
                };



                debug!("fakedns fakeip {:?} for {:?}",&ip,&domain);
                let mut resp = Message::new();

                // sets the response according to request
                // https://github.com/miekg/dns/blob/f515aa579d28efa1af67d9a62cc57f2dfe59da76/defaults.go#L15
                resp.set_id(req.id())
                    .set_message_type(MessageType::Response)
                    .set_op_code(req.op_code());

                if resp.op_code() == OpCode::Query {
                    resp.set_recursion_desired(req.recursion_desired())
                        .set_checking_disabled(req.checking_disabled());
                }
                resp.set_response_code(ResponseCode::NoError);
                if !req.queries().is_empty() {
                    resp.add_query(query.clone());
                }

                let mut ans = Record::new();
                ans.set_name(name.to_owned())
                    .set_rr_type(record_type)
                    .set_ttl(ttl)
                    .set_dns_class(class)
                    .set_rdata(ip);
                resp.add_answer(ans);
        /*        if query.query_type() == RecordType::A {
                    let mut ans = Record::new();
                    ans.set_name(raw_name.clone())
                        .set_rr_type(RecordType::A)
                        .set_ttl(self.ttl)
                        .set_dns_class(DNSClass::IN)
                        .set_rdata(RData::A(ip));
                    resp.add_answer(ans);
                }*/

                Ok(resp.to_vec()?)*/
    }

    pub fn is_fake_ip(&self, ip: &IpAddr) -> bool {
        let ip = match ip {
            IpAddr::V4(ip) => ip,
            _ => return false,
        };
        let ip = Self::ip_to_u32(ip);
        ip >= self.min_cursor && ip <= self.max_cursor
    }

    fn u32_to_ip(ip: u32) -> Ipv4Addr {
        Ipv4Addr::from(ip)
    }

    fn ip_to_u32(ip: &Ipv4Addr) -> u32 {
        BigEndian::read_u32(&ip.octets())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_u32_to_ip() {
        let ip1 = Ipv4Addr::new(127, 0, 0, 1);
        let ip2 = FakeDns::u32_to_ip(2130706433u32);
        assert_eq!(ip1, ip2);
    }

    #[test]
    fn test_ip_to_u32() {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let ip1 = FakeDns::ip_to_u32(&ip);
        let ip2 = 2130706433u32;
        assert_eq!(ip1, ip2);
    }
}
