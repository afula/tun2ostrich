use crate::app::dispatcher::Dispatcher;
use crate::app::fake_dns::{FakeDns, FakeDnsMode};
use crate::app::nat_manager::NatManager;
use byte_string::ByteStr;
use ipnet::IpNet;
use ipnet::Ipv4Net;
use log::{debug, error, info, trace, warn};
use protobuf::RepeatedField;
use smoltcp::wire::{IpProtocol, TcpPacket, UdpPacket};
#[cfg(unix)]
use std::os::unix::io::RawFd;
use std::{
    io::{self, ErrorKind},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex as TokioMutex;
use tokio::{io::AsyncReadExt, sync::mpsc, time};
use tun::{
    AsyncDevice, Configuration as TunConfiguration, Device as TunDevice, Error as TunError, Layer,
};

use crate::Runner;

use self::{
    ip_packet::IpPacket,
    sys::{write_packet_with_pi, IFF_PI_PREFIX_LEN},
    tcp::TcpTun,
    udp::UdpTun,
};

mod addr;
mod ip_packet;
mod sys;
mod tcp;
mod udp;
mod virt_device;

/*pub struct TunBuilder {
    // context: Arc<ServiceContext>,
    // balancer: PingBalancer,
    tun_config: TunConfiguration,
    udp_expiry_duration: Option<Duration>,
    udp_capacity: Option<usize>,
    // mode: Mode,
}

impl TunBuilder {
    pub fn new(context: Arc<ServiceContext>, balancer: PingBalancer) -> TunBuilder {
        TunBuilder {
            // context,
            // balancer,
            tun_config: TunConfiguration::default(),
            udp_expiry_duration: None,
            udp_capacity: None,
            // mode: Mode::TcpOnly,
        }
    }

    pub fn address(mut self, addr: IpNet) -> TunBuilder {
        self.tun_config.address(addr.addr()).netmask(addr.netmask());
        self
    }

    pub fn name(mut self, name: &str) -> TunBuilder {
        self.tun_config.name(name);
        self
    }

    #[cfg(unix)]
    pub fn file_descriptor(mut self, fd: RawFd) -> TunBuilder {
        self.tun_config.raw_fd(fd);
        self
    }

    pub fn udp_expiry_duration(mut self, udp_expiry_duration: Duration) -> TunBuilder {
        self.udp_expiry_duration = Some(udp_expiry_duration);
        self
    }

    pub fn udp_capacity(mut self, udp_capacity: usize) -> TunBuilder {
        self.udp_capacity = Some(udp_capacity);
        self
    }

    pub fn mode(mut self, mode: Mode) -> TunBuilder {
        self.mode = mode;
        self
    }

    pub async fn build(mut self) -> io::Result<Tun> {
        self.tun_config.layer(Layer::L3).up();

        #[cfg(any(target_os = "linux"))]
        self.tun_config.platform(|tun_config| {
            // IFF_NO_PI preventing excessive buffer reallocating
            tun_config.packet_information(false);
        });

        let device = match tun::create_as_async(&self.tun_config) {
            Ok(d) => d,
            Err(TunError::Io(err)) => return Err(err),
            Err(err) => return Err(io::Error::new(ErrorKind::Other, err)),
        };

        let (udp, udp_cleanup_interval, udp_keepalive_rx) = UdpTun::new(
            // self.context.clone(),
            // self.balancer.clone(),
            self.udp_expiry_duration,
            self.udp_capacity,
        );

        let tcp = TcpTun::new(
            // self.context,
            self.balancer,
            device.get_ref().mtu().unwrap_or(1500) as u32,
        );

        Ok(Tun {
            device,
            tcp,
            udp,
            // udp_cleanup_interval,
            // udp_keepalive_rx,
            // mode: self.mode,
        })
    }
}*/

pub fn tun_build(
    inbound_tag: String,
    tun_config: TunConfiguration,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
    fake_dns_mode: FakeDnsMode,
    fake_dns_filters: RepeatedField<String>,
) -> anyhow::Result<Runner> {
    /*    #[cfg(any(target_os = "ios", target_os = "android"))]
        let tun_address = Ipv4Addr::from_str("172.16.0.2").unwrap();
    #[cfg(any(target_os = "ios", target_os = "android"))]
        let tun_netmask = Ipv4Addr::from_str("255.255.255.0").unwrap();
    #[cfg(any(target_os = "ios", target_os = "android"))]
        let tun_name = "utun233";*/

    /*    #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        Ipv4Net::new(tun_address, tun_netmask_u32.leading_ones() as u8).expect("Ipv4Net::new");*/
    // let tun_config =  tun_config.clone();
    let device = match tun::create_as_async(&tun_config) {
        Ok(d) => d,
        Err(TunError::Io(err)) => {
            /*            error!("{}",anyhow::anyhow!("{:?}", err));
            return;*/
            return Err(anyhow::anyhow!("{:?}", err));
        }
        Err(err) => {
            error!("{}", anyhow::anyhow!("{:?}", err));
            // return ;
            return Err(anyhow::anyhow!("{:?}", err));
        }
    };
    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(fake_dns_mode)));

        for filter in fake_dns_filters.into_iter() {
            fakedns.lock().await.add_filter(filter);
        }

        let tun = Tun {
            device,
            tcp: TcpTun::new(
                dispatcher.clone(),
                nat_manager.clone(),
                fakedns.clone(),
                1500 as u32,
            ),
            udp: UdpTun::new(
                inbound_tag.clone(),
                dispatcher.clone(),
                nat_manager.clone(),
                fakedns.clone(),
            ),
            inbound_tag,
        };
        tun.run().await.unwrap();
    }))
}

pub struct Tun {
    device: AsyncDevice,
    tcp: TcpTun,
    udp: UdpTun,
    inbound_tag: String,
    // udp_cleanup_interval: Duration,
    // udp_keepalive_rx: mpsc::Receiver<SocketAddr>,
    // mode: Mode,
}

impl Tun {
    pub async fn run(mut self) -> io::Result<()> {
        let mtu = self.device.get_ref().mtu().expect("mtu");
        assert!(mtu > 0 && mtu as usize > IFF_PI_PREFIX_LEN);

        info!("tun device {}, mtu {}", self.device.get_ref().name(), mtu,);

        let mut packet_buffer = vec![0u8; 65536 + IFF_PI_PREFIX_LEN].into_boxed_slice();
        // let mut udp_cleanup_timer = time::interval(self.udp_cleanup_interval);

        loop {
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

                                if let Err(err) = self.handle_tun_frame(packet).await {
                                    error!("[TUN] handle IP frame failed, error: {}", err);
                                }
                            }

                            // UDP channel sent back
                            packet = self.udp.recv_packet() => {
                                if let Err(err) = write_packet_with_pi(&mut self.device, &packet.data).await {
                                    error!("[TUN] failed to set packet information, error: {}, {:?}", err, ByteStr::new(&packet.data));
                                } else {
                                    // trace!("[TUN] sent IP packet (UDP) {:?}", ByteStr::new(&packet.data));
                                }
                            }

            /*                // UDP cleanup expired associations
                            _ = udp_cleanup_timer.tick() => {
                                self.udp.cleanup_expired().await;
                            }

                            // UDP keep-alive associations
                            peer_addr_opt = self.udp_keepalive_rx.recv() => {
                                let peer_addr = peer_addr_opt.expect("UDP keep-alive channel closed unexpectly");
                                self.udp.keep_alive(&peer_addr).await;
                            }*/

                            // TCP channel sent back
                            packet = self.tcp.recv_packet() => {
                                if let Err(err) = write_packet_with_pi(&mut self.device, &packet).await {
                                    error!("[TUN] failed to set packet information, error: {}, {:?}", err, ByteStr::new(&packet));
                                } else {
                                    // trace!("[TUN] sent IP packet (TCP) {:?}", ByteStr::new(&packet));
                                }
                            }
                        }
        }
    }

    async fn handle_tun_frame(&mut self, frame: &[u8]) -> smoltcp::Result<()> {
        let packet = match IpPacket::new_checked(frame)? {
            Some(packet) => packet,
            None => {
                warn!("unrecognized IP packet {:?}", ByteStr::new(frame));
                return Ok(());
            }
        };

        match packet.protocol() {
            IpProtocol::Tcp => {
                /*                if !self.mode.enable_tcp() {
                                    trace!("received TCP packet but mode is {}, throwing away", self.mode);
                                    return Ok(());
                                }
                */
                let tcp_packet = match TcpPacket::new_checked(packet.payload()) {
                    Ok(p) => p,
                    Err(err) => {
                        error!(
                            "invalid TCP packet err: {}, src_ip: {}, dst_ip: {}, payload: {:?}",
                            err,
                            packet.src_addr(),
                            packet.dst_addr(),
                            ByteStr::new(packet.payload())
                        );
                        return Ok(());
                    }
                };

                let src_port = tcp_packet.src_port();
                let dst_port = tcp_packet.dst_port();

                let src_addr = SocketAddr::new(packet.src_addr(), src_port);
                let dst_addr = SocketAddr::new(packet.dst_addr(), dst_port);

                /*                trace!(
                    "[TUN] TCP packet {} -> {} {}",
                    src_addr,
                    dst_addr,
                    tcp_packet
                );*/

                // TCP first handshake packet.
                if let Err(err) = self
                    .tcp
                    .handle_packet(src_addr, dst_addr, &tcp_packet, self.inbound_tag.clone())
                    .await
                {
                    error!(
                        "handle TCP packet failed, error: {}, {} <-> {}, packet: {:?}",
                        err, src_addr, dst_addr, tcp_packet
                    );
                }

                self.tcp.drive_interface_state(frame).await;
            }
            IpProtocol::Udp => {
                /*                if !self.mode.enable_udp() {
                    trace!("received UDP packet but mode is {}, throwing away", self.mode);
                    return Ok(());
                }*/

                let udp_packet = match UdpPacket::new_checked(packet.payload()) {
                    Ok(p) => p,
                    Err(err) => {
                        error!(
                            "invalid UDP packet err: {}, src_ip: {}, dst_ip: {}, payload: {:?}",
                            err,
                            packet.src_addr(),
                            packet.dst_addr(),
                            ByteStr::new(packet.payload())
                        );
                        return Ok(());
                    }
                };

                let src_port = udp_packet.src_port();
                let dst_port = udp_packet.dst_port();

                let src_addr = SocketAddr::new(packet.src_addr(), src_port);
                let dst_addr = SocketAddr::new(packet.dst_addr(), dst_port);

                let payload = udp_packet.payload();
                /*                trace!(
                    "[TUN] UDP packet {} -> {} {}",
                    src_addr,
                    dst_addr,
                    udp_packet
                );*/

                if let Err(err) = self.udp.handle_packet(src_addr, dst_addr, payload).await {
                    error!(
                        "handle UDP packet failed, err: {}, packet: {:?}",
                        err, udp_packet
                    );
                }
            }
            IpProtocol::Icmp | IpProtocol::Icmpv6 => {
                // ICMP is handled by TCP's Interface.
                // smoltcp's interface will always send replies to EchoRequest
                self.tcp.drive_interface_state(frame).await;
            }
            _ => {
                debug!("IP packet ignored (protocol: {:?})", packet.protocol());
                return Ok(());
            }
        }

        Ok(())
    }
}
