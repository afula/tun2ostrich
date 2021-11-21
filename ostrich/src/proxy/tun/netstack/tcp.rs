use std::{
    io::{self, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use etherparse::TcpHeader;
use ipnet::IpNet;
use log::{debug, error, trace};
use lru_time_cache::LruCache;
use tokio::sync::Mutex as TokioMutex;
use crate::app::dispatcher::Dispatcher;
use crate::app::nat_manager::NatManager;
use tokio::net::TcpListener;
use tokio::{net::TcpStream, sync::Mutex, task::JoinHandle, time};
use crate::app::fake_dns::FakeDns;

use crate::session::{Network, Session, SocksAddr};

struct TcpAddressTranslator {
    connections: LruCache<SocketAddr, TcpConnection>,
    mapping: LruCache<(SocketAddr, SocketAddr), SocketAddr>,
}

impl TcpAddressTranslator {
    fn new() -> TcpAddressTranslator {
        TcpAddressTranslator {
            connections: LruCache::with_expiry_duration(Duration::from_secs(24 * 60 * 60)),
            mapping: LruCache::with_expiry_duration(Duration::from_secs(24 * 60 * 60)),
        }
    }
}

pub struct TcpTun {
    tcp_daddr: SocketAddr,
    free_addrs: Vec<IpAddr>,
    translator: Arc<Mutex<TcpAddressTranslator>>,
    abortable: JoinHandle<anyhow::Result<()>>,
    // dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
}

impl Drop for TcpTun {
    fn drop(&mut self) {
        self.abortable.abort();
    }
}

impl TcpTun {
    pub async fn new(
        inbound_tag: String,
        tun_network: IpNet,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> io::Result<TcpTun> {
        let mut hosts = tun_network.hosts();
        let tcp_daddr = match hosts.next() {
            Some(d) => d,
            None => {
                return Err(io::Error::new(
                    ErrorKind::Other,
                    "tun network doesn't have any hosts",
                ))
            }
        };

        // Take up to 10 IPs as saddr for NAT allocating
        let free_addrs = hosts.take(1024).collect::<Vec<IpAddr>>();
        if free_addrs.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "tun network doesn't have enough free addresses",
            ));
        }

        let listener = TcpListener::bind(&SocketAddr::new(tcp_daddr, 0)).await?;
        let tcp_daddr = listener.local_addr()?;

        debug!("tun tcp listener bind {}", tcp_daddr);

        let translator = Arc::new(Mutex::new(TcpAddressTranslator::new()));

        let abortable = {
            let translator = translator.clone();
            tokio::spawn(TcpTun::tunnel(
                inbound_tag,
                listener,
                translator,
                dispatcher,
                fakedns
            ))
        };

        Ok(TcpTun {
            tcp_daddr,
            free_addrs,
            translator,
            abortable,
            // dispatcher,
            nat_manager,
        })
    }

    pub async fn handle_packet(
        &mut self,
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        tcp_header: &TcpHeader,
    ) -> io::Result<Option<(SocketAddr, SocketAddr)>> {
        let TcpAddressTranslator {
            ref mut connections,
            ref mut mapping,
        } = *(self.translator.lock().await);

        let (conn, is_reply) = if tcp_header.syn && !tcp_header.ack {
            // 1st SYN, creating a new connection
            // Allocate a `saddr` for it
            let saddr = loop {
                let addr_idx = rand::random::<usize>() % self.free_addrs.len();
                let port = rand::random::<u16>() % (65535 - 1024) + 1024;

                let addr = SocketAddr::new(self.free_addrs[addr_idx], port);
                if !connections.contains_key(&addr) {
                    trace!(
                        "allocated tcp addr {} for {} -> {}",
                        addr,
                        src_addr,
                        dst_addr
                    );

                    // Create one in the connection map.
                    connections.insert(
                        addr,
                        TcpConnection {
                            saddr: src_addr,
                            daddr: dst_addr,
                            faked_saddr: addr,
                            state: TcpState::Established,
                        },
                    );

                    // Record the fake address mapping
                    mapping.insert((src_addr, dst_addr), addr);

                    break addr;
                }
            };

            (connections.get_mut(&saddr).unwrap(), false)
        } else {
            // Find if it is an existed connection, ignore it otherwise
            match mapping.get(&(src_addr, dst_addr)) {
                Some(saddr) => match connections.get_mut(saddr) {
                    Some(c) => (c, false),
                    None => {
                        debug!("unknown tcp connection {} -> {}", src_addr, dst_addr);
                        return Ok(None);
                    }
                },
                None => {
                    // Check if it is a reply packet
                    match connections.get_mut(&dst_addr) {
                        Some(c) => (c, true),
                        None => {
                            debug!("unknown tcp connection {} -> {}", src_addr, dst_addr);
                            return Ok(None);
                        }
                    }
                }
            }
        };

        let (trans_saddr, trans_daddr) = if is_reply {
            trace!("TCP {} <- {} {:?}", conn.saddr, conn.daddr, tcp_header);
            (conn.daddr, conn.saddr)
        } else {
            trace!("TCP {} -> {} {:?}", conn.saddr, conn.daddr, tcp_header);
            (conn.faked_saddr, self.tcp_daddr)
        };

        if tcp_header.rst || (tcp_header.ack && conn.state == TcpState::LastAck) {
            // Connection closed.
            trace!("tcp connection closed {} -> {}", conn.saddr, conn.daddr);

            mapping.remove(&(src_addr, dst_addr));
            let faked_saddr = conn.faked_saddr;
            connections.remove(&faked_saddr);
        } else if tcp_header.fin {
            match conn.state {
                TcpState::Established => conn.state = TcpState::FinWait,
                TcpState::FinWait => conn.state = TcpState::LastAck,
                _ => {}
            }
        }

        Ok(Some((trans_saddr, trans_daddr)))
    }

    async fn tunnel(
        inbound_tag: String,
        listener: TcpListener,
        translator: Arc<Mutex<TcpAddressTranslator>>,
        dispatcher: Arc<Dispatcher>,
        fakedns: Arc<TokioMutex<FakeDns>>,
    ) -> anyhow::Result<()> {
        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(s) => s,
                Err(err) => {
                    error!("accept failed, error: {}", err);
                    time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Try to translate
            let (saddr, daddr) = {
                let mut translator = translator.lock().await;
                match translator.connections.get(&peer_addr) {
                    Some(c) => (c.saddr, c.daddr),
                    None => {
                        error!("unknown connection from {}", peer_addr);
                        continue;
                    }
                }
            };
            debug!("establishing tcp tunnel {} -> {}", saddr, daddr);

            let inbound_tag = inbound_tag.clone();
            let dispatcher = dispatcher.clone();
            let fakedns = fakedns.clone();
            let mut sess = Session {
                network: Network::Tcp,
                source: saddr,
                local_addr: daddr,
                destination: SocksAddr::Ip(daddr),
                inbound_tag,
                ..Default::default()
            };

            if fakedns.lock().await.is_fake_ip(&daddr.ip()) {
                if let Some(domain) = fakedns
                    .lock()
                    .await
                    .query_domain(&&daddr.ip())
                {
                    sess.destination =
                        SocksAddr::Domain(domain, daddr.port());
                } else {
                    // Although requests targeting fake IPs are assumed
                    // never happen in real network traffic, which are
                    // likely caused by poisoned DNS cache records, we
                    // still have a chance to sniff the request domain
                    // for TLS traffic in dispatcher.
                    if daddr.port() != 443 {
                        return Err(anyhow::anyhow!("fake ip with error: {:?}",&daddr))
                    }
                }
            }
            // tokio::spawn(async move {
            //     if let Err(err) = handle_redir_client(balancer, stream, peer_addr, daddr).await {
            //         debug!("TCP redirect client, error: {:?}", err);
            //     }
            // });

            tokio::spawn(async move {

                dispatcher.dispatch_tcp(&mut sess, stream).await;
            });
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum TcpState {
    /// TCP state `ESTABLISHED`
    ///
    /// When receiving the first SYN then the state will be set to `ESTABLISHED`.
    /// The detailed state like (SYN_SEND, SYN_RCVD) will be handled properly by the `TcpListener`.
    Established,
    /// When receiving from the first FIN will be transferred from Established
    FinWait,
    /// When receiving the last ACK of FIN will be transferred from FinWait
    LastAck,
}

struct TcpConnection {
    saddr: SocketAddr,
    daddr: SocketAddr,
    faked_saddr: SocketAddr,
    state: TcpState,
}
