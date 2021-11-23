#[cfg(unix)]
use std::os::unix::io::RawFd;
use std::{
    io::{self, Cursor, ErrorKind},
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::{FakeDns, FakeDnsMode};
use crate::app::nat_manager::{NatManager, UdpPacket};
use crate::Runner;
use byte_string::ByteStr;
use bytes::BytesMut;
use etherparse::{IpHeader, PacketHeaders, ReadError, TransportHeader};
use futures::future;
use ipnet::{IpNet, Ipv4Net};
use log::{error, info, trace, warn};
use protobuf::RepeatedField;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tun::{AsyncDevice, Configuration as TunConfiguration, Device, Error as TunError, Layer};

use self::{
    sys::{set_route_configuration, write_packet_with_pi, IFF_PI_PREFIX_LEN},
    tcp::TcpTun,
    udp::UdpTun,
};

mod sys;
mod tcp;
mod udp;

pub fn tun_build(
    inbound_tag: String,
    tun_config: &TunConfiguration,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    fake_dns_mode: FakeDnsMode,
    fake_dns_filters: RepeatedField<String>,
) -> anyhow::Result<Runner> {
    let device = match tun::create_as_async(tun_config) {
        Ok(d) => d,
        Err(TunError::Io(err)) => return Err(anyhow::anyhow!("{:?}", err)),
        Err(err) => return Err(anyhow::anyhow!("{:?}", err)),
    };

    let tun_address = match device.get_ref().address() {
        Ok(t) => t,
        Err(err) => {
            error!(
                "tun device doesn't have address, error: {}, set it by tun_interface_address",
                err
            );
            return Err(anyhow::anyhow!("{:?}", err));
        }
    };

    let tun_netmask = match device.get_ref().netmask() {
        Ok(m) => m,
        Err(err) => {
            error!(
                "tun device doesn't have netmask, error: {}, set it by tun_interface_address",
                err
            );
            return Err(anyhow::anyhow!("{:?}", err));
        }
    };

    let tun_name = device.get_ref().name();

    trace!(
        "tun {}, address: {}, netmask: {}",
        tun_name,
        tun_address,
        tun_netmask
    );

    /*        if let Err(err) = set_route_configuration(device.get_ref()).await {
                warn!(
                    "failed to set system route for {}, consider set it manually, error: {}",
                    tun_name, err
                );
            }
    */
    let tun_netmask_u32: u32 = tun_netmask.into();

    let tun_network =
        Ipv4Net::new(tun_address, tun_netmask_u32.leading_ones() as u8).expect("Ipv4Net::new");

    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(fake_dns_mode)));

        for filter in fake_dns_filters.into_iter() {
            fakedns.lock().await.add_filter(filter);
        }

        let tun = Tun {
            device,
            tcp: TcpTun::new(
                inbound_tag.clone(),
                tun_network.into(),
                dispatcher.clone(),
                nat_manager.clone(),
                fakedns.clone(),
            )
            .await
            .unwrap(),
            udp: UdpTun::new(
                inbound_tag.clone(),
                dispatcher.clone(),
                nat_manager.clone(),
                fakedns,
            ),
        };
        tun.run().await.unwrap();
    }))
}
pub struct Tun {
    device: AsyncDevice,
    tcp: TcpTun,
    udp: UdpTun,
    // mode: Mode,
}

impl Tun {
    pub async fn run(mut self) -> anyhow::Result<()> {
        let mtu = self.device.get_ref().mtu().expect("mtu");
        assert!(mtu > 0 && mtu as usize > IFF_PI_PREFIX_LEN);

        info!(
            "tun device {}, address {}, netmask {}, mtu {}",
            self.device.get_ref().name(),
            self.device.get_ref().address().expect("address"),
            self.device.get_ref().netmask().expect("netmask"),
            mtu,
        );

        let mut packet_buffer = vec![0u8; mtu as usize + IFF_PI_PREFIX_LEN].into_boxed_slice();

        loop {
            #[inline(always)]
            async fn udp_recv_packet(udp: &mut UdpTun) -> UdpPacket {
                udp.recv_packet().await
            }

            tokio::select! {
                // tun device
                n = self.device.read(&mut packet_buffer) => {
                    let n = n?;

                    if n <= IFF_PI_PREFIX_LEN {
                        error!(
                            "[TUN] packet too short, packet: {:?}",
                            ByteStr::new(&packet_buffer[..n])
                        );
                        continue;
                    }

                    let packet = &mut packet_buffer[IFF_PI_PREFIX_LEN..n];
                    // trace!("[TUN] received IP packet {:?}", ByteStr::new(packet));

                    if self.handle_packet(packet).await? {
                        self.device.write_all(&packet_buffer[..n]).await?;
                    }
                }

                // channel sent back
                packet = udp_recv_packet(&mut self.udp) => {
                    if let Err(err) = write_packet_with_pi(&mut self.device, &packet.data).await {
                        error!("failed to set packet information, error: {}, {:?}", err, ByteStr::new(&packet.data));
                    }
                }
            }
        }
        /*        #[inline(always)]
        async fn udp_recv_packet(udp: &mut UdpTun) -> UdpPacket {
            udp.recv_packet().await
        }
        let s2t = Box::pin(async  {
            loop {
                // tun device
                let n = self.device.read(&mut packet_buffer).await?;
                // let n = n?;

                if n <= IFF_PI_PREFIX_LEN {
                    error!(
                        "[TUN] packet too short, packet: {:?}",
                        ByteStr::new(&packet_buffer[..n])
                    );
                    continue;
                }

                let packet = &mut packet_buffer[IFF_PI_PREFIX_LEN..n];
                trace!("[TUN] received IP packet {:?}", ByteStr::new(packet));

                if self.handle_packet(packet).await? {
                    self.device.write_all(&packet_buffer[..n]).await?;
                }
            }
            Ok(()) as anyhow::Result<()>
        });

        let t2s = Box::pin(async  {
            // channel sent back
            loop {
                let packet = udp_recv_packet(&mut self.udp).await;
                if let Err(err) = write_packet_with_pi(&mut self.device, &packet.data).await {
                    error!(
                        "failed to set packet information, error: {}, {:?}",
                        err,
                        ByteStr::new(&packet.data)
                    );
                }
            }
        });

        info!("tun inbound started");
        futures::future::select(t2s, s2t).await;
        info!("tun inbound exited");*/
        Ok(())
    }

    async fn handle_packet(&mut self, packet: &mut [u8]) -> io::Result<bool> {
        let mut ph = match PacketHeaders::from_ip_slice(packet) {
            Ok(ph) => ph,
            Err(ReadError::IoError(err)) => return Err(err),
            Err(err) => {
                error!(
                    "invalid IP packet, error: {:?}, {:?}",
                    err,
                    ByteStr::new(packet)
                );
                return Err(io::Error::new(ErrorKind::Other, err));
            }
        };

        let payload_len = ph.payload.len();

        let mut ip_header = match ph.ip {
            Some(ref mut i) => i,
            None => {
                error!("unrecognized ethernet packet {:?}", ph);
                return Err(io::Error::new(
                    ErrorKind::Other,
                    "unrecognized ethernet packet",
                ));
            }
        };

        let (src_ip, dst_ip) = match *ip_header {
            IpHeader::Version4(ref v4) => (
                Ipv4Addr::from(v4.source).into(),
                Ipv4Addr::from(v4.destination).into(),
            ),
            IpHeader::Version6(ref v6) => (
                Ipv6Addr::from(v6.source).into(),
                Ipv6Addr::from(v6.destination).into(),
            ),
        };

        match ph.transport {
            Some(TransportHeader::Tcp(ref mut tcp_header)) => {
                let src_addr = SocketAddr::new(src_ip, tcp_header.source_port);
                let dst_addr = SocketAddr::new(dst_ip, tcp_header.destination_port);

                let (mod_src_addr, mod_dst_addr) =
                    match self.tcp.handle_packet(src_addr, dst_addr, tcp_header).await {
                        Ok(Some(a)) => a,
                        Ok(None) => return Ok(false),
                        Err(err) => {
                            error!("handle TCP/IP packet failed, error: {}", err);
                            return Ok(false);
                        }
                    };

                // Replaces IP_HEADER, TRANSPORT_HEADER directly into packet
                match (mod_src_addr, &mut ip_header) {
                    (SocketAddr::V4(v4addr), IpHeader::Version4(v4ip)) => {
                        v4ip.source = v4addr.ip().octets()
                    }
                    (SocketAddr::V6(v6addr), IpHeader::Version6(v6ip)) => {
                        v6ip.source = v6addr.ip().octets()
                    }
                    _ => unreachable!("modified saddr not match"),
                }
                tcp_header.source_port = mod_src_addr.port();
                match (mod_dst_addr, &mut ip_header) {
                    (SocketAddr::V4(v4addr), IpHeader::Version4(v4ip)) => {
                        v4ip.destination = v4addr.ip().octets()
                    }
                    (SocketAddr::V6(v6addr), IpHeader::Version6(v6ip)) => {
                        v6ip.destination = v6addr.ip().octets()
                    }
                    _ => unreachable!("modified daddr not match"),
                }
                tcp_header.destination_port = mod_dst_addr.port();
                match ip_header {
                    IpHeader::Version4(v4) => {
                        tcp_header.checksum = tcp_header
                            .calc_checksum_ipv4(v4, ph.payload)
                            .expect("calc_checksum_ipv4")
                    }
                    IpHeader::Version6(v6) => {
                        tcp_header.checksum = tcp_header
                            .calc_checksum_ipv6(v6, ph.payload)
                            .expect("calc_checksum_ipv6")
                    }
                }

                let (headers, _) = packet.split_at_mut(packet.len() - payload_len);
                let mut cursor = Cursor::new(headers);

                ip_header.write(&mut cursor).expect("ip_header.write");
                tcp_header.write(&mut cursor).expect("tcp_header.write");

                Ok(true)
            }
            Some(TransportHeader::Udp(ref udp_header)) => {
                // UDP proxies directly

                let src_addr = SocketAddr::new(src_ip, udp_header.source_port);
                let dst_addr = SocketAddr::new(dst_ip, udp_header.destination_port);

                if let Err(err) = self.udp.handle_packet(src_addr, dst_addr, ph.payload).await {
                    error!("handle UDP/IP packet failed, error: {}", err);
                }

                Ok(false)
            }
            None => {
                trace!("no transport layer in ethernet packet {:?}", ph);
                Ok(false)
            }
        }
    }
}
