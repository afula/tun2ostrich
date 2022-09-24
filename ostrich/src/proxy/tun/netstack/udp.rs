use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::FakeDns;
use crate::app::nat_manager::{NatManager, UdpPacket};
use crate::session::{DatagramSource, Network, Session, SocksAddr};
use log::warn;
use std::{
    io::{self, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use etherparse::PacketBuilder;
use log::{debug, trace};
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;

pub struct UdpTun {
    // tun_rx: mpsc::Receiver<BytesMut>,
    // manager: UdpAssociationManager<UdpTunInboundWriter>,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    udp_tun_rx: Receiver<UdpPacket>,
    udp_processor_tx: Sender<UdpPacket>,
    fakedns: Arc<FakeDns>,
}

impl UdpTun {
    pub fn new(
        inbound_tag: String,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
        fakedns: Arc<FakeDns>,
    ) -> UdpTun {
        let (udp_tun_tx, udp_tun_rx) = channel(1024);

        let (udp_processor_tx, mut udp_processor_rx): (Sender<UdpPacket>, Receiver<UdpPacket>) =
            channel(1024);
        let udp_nat_manager = nat_manager.clone();
        let udp_fakedns = fakedns.clone();
        let fakedns_dispatcher = dispatcher.clone();
        let udp_processor_tx_nat = udp_processor_tx.clone();
        tokio::spawn(async move {
            while let Some(pkt) = udp_processor_rx.recv().await {
                let dst_addr = pkt.dst_addr;

                let src_addr = match pkt.src_addr {
                    SocksAddr::Ip(a) => SocksAddr::Ip(a),
                    SocksAddr::Domain(domain, port) => {
                        if let Some(ip) = udp_fakedns.query_fake_ip(&domain).await {
                            SocksAddr::Ip(SocketAddr::new(ip, port))
                        } else {
                            warn!(
                                "Received datagram with source address {}:{} without paired fake IP found.",
                                &domain, &port
                            );
                            continue;
                        }
                    }
                };

                debug!("udp dest addr: {:?}", &dst_addr);
                if dst_addr.port() == 53 {
                    debug!("udp dns query: {:?}", &dst_addr);
                    match udp_fakedns.generate_fake_response(&pkt.data).await {
                        Ok(resp) => {
                            // send_udp(lwip_lock.clone(), &dst_addr, &src_addr, pcb, resp.as_ref());
                            let packet = UdpPacket {
                                data: resp,
                                dst_addr,
                                src_addr,
                            };
                            udp_tun_tx
                                .send(packet)
                                .await
                                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
                            continue;
                        }
                        Err(err) => {
                            trace!("generate fake ip failed: {}", err);
                        }
                    }
                }

                // We're sending UDP packets to a fake IP, and there should be a paired domain,
                // that said, the application connects a UDP socket with a domain address.
                // It also means the back packets on this UDP session shall only come from a
                // single source address.

                // let socks_dst_addr = SocksAddr::Ip(dst_addr);
                let socks_dst_addr = if udp_fakedns.is_fake_ip(&dst_addr.ip().unwrap()).await {
                    if let Some(domain) = udp_fakedns.query_domain(&dst_addr.ip().unwrap()).await {
                        SocksAddr::Domain(domain, dst_addr.port())
                    } else {
                        log::debug!(
                            "No paired domain found for this fake IP: {}, datagram is rejected.",
                            &dst_addr.ip().unwrap()
                        );
                        continue;
                    }
                } else {
                    // SocksAddr::Ip(dst_addr)
                    dst_addr
                };

                let dgram_src = DatagramSource::new(
                    SocketAddr::new(src_addr.ip().unwrap(), src_addr.port()),
                    None,
                );
                /*                if !nat_manager.contains_key(&dgram_src).await {
                    let sess = Session {
                        network: Network::Udp,
                        source: dgram_src.address,
                        destination: socks_dst_addr.clone(),
                        inbound_tag: inbound_tag.clone(),
                        ..Default::default()
                    };
                    nat_manager
                        .send(&sess, dgram_src, udp_processor_tx.clone())
                        .await;
                    // Note that subsequent packets on this session may have different
                    // destination addresses.
                    debug!(
                        "added udp session {} -> {}:{} ({})",
                        &dgram_src,
                        &dst_addr.ip(),
                        &dst_addr.port(),
                        nat_manager.size().await,
                    );
                }*/

                let pkt = UdpPacket {
                    data: pkt.data,
                    src_addr: SocksAddr::Ip(dgram_src.address),
                    dst_addr: socks_dst_addr.clone(),
                };
                nat_manager
                    .send(&dgram_src, &inbound_tag.clone(), &udp_processor_tx_nat, pkt)
                    .await;
            }
            Ok(()) as anyhow::Result<()>
        });
        UdpTun {
            dispatcher,
            nat_manager: udp_nat_manager,
            udp_tun_rx,
            udp_processor_tx: udp_processor_tx.clone(),
            fakedns: fakedns,
        }
    }

    pub async fn handle_packet(
        &mut self,
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        payload: &[u8],
    ) -> anyhow::Result<()> {
        trace!(
            "UDP {} -> {} payload.size: {} bytes",
            src_addr,
            dst_addr,
            payload.len()
        );
        let packet = UdpPacket {
            data: payload.to_vec(),
            src_addr: SocksAddr::Ip(src_addr),
            dst_addr: SocksAddr::Ip(dst_addr),
        };
        self.udp_processor_tx
            .send(packet)
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(())
    }

    pub async fn recv_packet(&mut self) -> UdpPacket {
        match self.udp_tun_rx.recv().await {
            Some(udp_packet) => {
                let data = udp_packet.data;

                let socks_src_addr = udp_packet.src_addr.clone();
                let dst_addr = SocketAddr::new(
                    udp_packet.dst_addr.ip().unwrap(),
                    udp_packet.dst_addr.port(),
                );
                let src_addr = match socks_src_addr {
                    SocksAddr::Ip(ref a) => {
                        if dst_addr.is_ipv4() {
                            match a.ip().to_canonical() {
                                IpAddr::V4(ip4) => SocketAddr::new(IpAddr::V4(ip4), a.port()),
                                IpAddr::V6(ip6) => {
                                    unreachable!("unexpected dst addr");
                                }
                            }
                        } else {
                            a.to_owned()
                        }
                    }
                    SocksAddr::Domain(domain, port) => {
                        unreachable!(
                            "unexpected domain src addr {}:{} without paired fake IP",
                            &domain, &port
                        );
                    } /*                    // If the socket gives us a domain source address,
                      // we assume there must be a paired fake IP, otherwise
                      // we have no idea how to deal with it.
                      SocksAddr::Domain(domain, port) => {
                          // TODO we're doing this for every packet! optimize needed
                          // trace!("downlink querying fake ip for domain {}", &domain);
                          if let Some(ip) = self.fakedns.lock().await.query_fake_ip(&domain) {
                              SocketAddr::new(ip, port)
                          } else {
                              unreachable!(
                                  "unexpected domain src addr {}:{} without paired fake IP",
                                  &domain, &port
                              );
                          }
                      }*/
                };
                let packet = match (src_addr, dst_addr) {
                    (SocketAddr::V4(peer), SocketAddr::V4(remote)) => {
                        let builder =
                            PacketBuilder::ipv4(remote.ip().octets(), peer.ip().octets(), 20)
                                .udp(remote.port(), peer.port());

                        let packet = BytesMut::with_capacity(builder.size(data.len()));
                        let mut packet_writer = packet.writer();
                        builder
                            .write(&mut packet_writer, data.as_slice())
                            .expect("PacketBuilder::write");

                        packet_writer.into_inner()
                    }
                    (SocketAddr::V6(peer), SocketAddr::V6(remote)) => {
                        let builder =
                            PacketBuilder::ipv6(remote.ip().octets(), peer.ip().octets(), 20)
                                .udp(remote.port(), peer.port());

                        let packet = BytesMut::with_capacity(builder.size(data.len()));
                        let mut packet_writer = packet.writer();
                        builder
                            .write(&mut packet_writer, data.as_slice())
                            .expect("PacketBuilder::write");

                        packet_writer.into_inner()
                    }
                    _ => {
                        warn!("{} = {}", src_addr, dst_addr);
                        unreachable!("recv_packet")
                    }
                };
                UdpPacket {
                    data: packet.to_vec(),
                    src_addr: udp_packet.src_addr.to_owned(),
                    dst_addr: udp_packet.dst_addr.to_owned(),
                }
            }
            None => unreachable!("channel closed unexpectedly"),
        }
    }

    /*    #[inline(always)]
    pub async fn cleanup_expired(&mut self) {
        self.manager.cleanup_expired().await;
    }

    #[inline(always)]
    pub async fn keep_alive(&mut self, peer_addr: &SocketAddr) {
        self.manager.keep_alive(peer_addr).await;
    }*/
}

/*#[derive(Clone)]
struct UdpTunInboundWriter {
    tun_tx: mpsc::Sender<BytesMut>,
}

impl UdpTunInboundWriter {
    fn new(tun_tx: mpsc::Sender<BytesMut>) -> UdpTunInboundWriter {
        UdpTunInboundWriter { tun_tx }
    }
}

#[async_trait]
impl UdpInboundWrite for UdpTunInboundWriter {
    async fn send_to(&self, peer_addr: SocketAddr, remote_addr: &Address, data: &[u8]) -> io::Result<()> {
        let addr = match *remote_addr {
            Address::SocketAddress(sa) => {
                // Try to convert IPv4 mapped IPv6 address if server is running on dual-stack mode
                match sa {
                    SocketAddr::V4(..) => sa,
                    SocketAddr::V6(ref v6) => match to_ipv4_mapped(v6.ip()) {
                        Some(v4) => SocketAddr::new(IpAddr::from(v4), v6.port()),
                        None => sa,
                    },
                }
            }
            Address::DomainNameAddress(..) => {
                let err = io::Error::new(
                    ErrorKind::InvalidInput,
                    "tun destination must not be an domain name address",
                );
                return Err(err);
            }
        };

        let packet = match (peer_addr, addr) {
            (SocketAddr::V4(peer), SocketAddr::V4(remote)) => {
                let builder =
                    PacketBuilder::ipv4(remote.ip().octets(), peer.ip().octets(), 20).udp(remote.port(), peer.port());

                let packet = BytesMut::with_capacity(builder.size(data.len()));
                let mut packet_writer = packet.writer();
                builder.write(&mut packet_writer, data).expect("PacketBuilder::write");

                packet_writer.into_inner()
            }
            (SocketAddr::V6(peer), SocketAddr::V6(remote)) => {
                let builder =
                    PacketBuilder::ipv6(remote.ip().octets(), peer.ip().octets(), 20).udp(remote.port(), peer.port());

                let packet = BytesMut::with_capacity(builder.size(data.len()));
                let mut packet_writer = packet.writer();
                builder.write(&mut packet_writer, data).expect("PacketBuilder::write");

                packet_writer.into_inner()
            }
            _ => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "source and destination type unmatch",
                ));
            }
        };

        self.tun_tx.send(packet).await.expect("tun_tx::send");
        Ok(())
    }
}
*/
