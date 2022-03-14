use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::*;
use protobuf::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex as TokioMutex;
use tun::{
    self,
    Device, TunPacket
};

use crate::{
    app::dispatcher::Dispatcher,
    app::fake_dns::{FakeDns, FakeDnsMode},
    app::nat_manager::NatManager,
    config::{Inbound, TunInboundSettings},
    option, Runner,
};

use super::netstack::NetStack;

const MTU: usize = 1500;

pub fn new(
    inbound: Inbound,
    dispatcher: Arc<Dispatcher>,
    nat_manager: Arc<NatManager>,
) -> Result<Runner> {
    let settings = TunInboundSettings::parse_from_bytes(&inbound.settings)?;

    let mut cfg = tun::Configuration::default();
    if settings.fd >= 0 {
        cfg.raw_fd(settings.fd);
    } else if settings.auto {
        cfg.name(&*option::DEFAULT_TUN_NAME)
            .address(&*option::DEFAULT_TUN_IPV4_ADDR)
            .destination(&*option::DEFAULT_TUN_IPV4_GW)
            .mtu(1500);

        #[cfg(not(any(
            target_arch = "mips",
            target_arch = "mips64",
            target_arch = "mipsel",
            target_arch = "mipsel64",
        )))]
        {
            cfg.netmask(&*option::DEFAULT_TUN_IPV4_MASK);
        }

        cfg.up();
    } else {
        cfg.name(settings.name)
            .address(settings.address)
            .destination(settings.gateway)
            .mtu(settings.mtu);

        #[cfg(not(any(
            target_arch = "mips",
            target_arch = "mips64",
            target_arch = "mipsel",
            target_arch = "mipsel64",
        )))]
        {
            cfg.netmask(settings.netmask);
        }

        cfg.up();
    }

    // FIXME it's a bad design to have 2 lists in config while we need only one
    let fake_dns_exclude = settings.fake_dns_exclude;
    let fake_dns_include = settings.fake_dns_include;
    if !fake_dns_exclude.is_empty() && !fake_dns_include.is_empty() {
        return Err(anyhow!(
            "fake DNS run in either include mode or exclude mode"
        ));
    }
    let (fake_dns_mode, fake_dns_filters) = if !fake_dns_include.is_empty() {
        (FakeDnsMode::Include, fake_dns_include)
    } else {
        (FakeDnsMode::Exclude, fake_dns_exclude)
    };
    if settings.auto {
        assert!(settings.fd == -1, "tun-auto is not compatible with tun-fd");
    }

    #[cfg(all(
    feature = "inbound-tun",
    any(
    target_os = "ios",
    target_os = "android",
    target_os = "macos",
    target_os = "linux",
    )
    ))]{
        let tun = tun::create_as_async(&cfg).map_err(|e| anyhow!("create tun failed: {}", e)).expect("cant create tun device");

    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(fake_dns_mode)));

        for filter in fake_dns_filters.into_iter() {
            fakedns.lock().await.add_filter(filter);
        }

        let stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);

        let mtu = tun.get_ref().mtu().unwrap_or(MTU as i32);
        let framed = tun.into_framed();
        let (mut tun_sink, mut tun_stream) = framed.split();
        let (mut stack_reader, mut stack_writer) = io::split(stack);

        let s2t = Box::pin(async move {
            let mut buf = vec![0; mtu as usize];
            loop {
                match stack_reader.read(&mut buf).await {
                    Ok(0) => {
                        debug!("read stack eof");
                        return;
                    }
                    Ok(n) => match tun_sink.send(TunPacket::new((&buf[..n]).to_vec())).await {
                        Ok(_) => (),
                        Err(e) => {
                            warn!("send pkt to tun failed: {}", e);
                            return;
                        }
                    },
                    Err(err) => {
                        warn!("read stack failed {:?}", err);
                        return;
                    }
                }
            }
        });

        let t2s = Box::pin(async move {
            while let Some(packet) = tun_stream.next().await {
                match packet {
                    Ok(packet) => match stack_writer.write(packet.get_bytes()).await {
                        Ok(_) => (),
                        Err(e) => {
                            warn!("write pkt to stack failed: {}", e);
                            return;
                        }
                    },
                    Err(err) => {
                        warn!("read tun failed {:?}", err);
                        return;
                    }
                }
            }
        });

                info!("tun inbound started");
                futures::future::select(t2s, s2t).await;
                info!("tun inbound exited");
    }))}

    #[cfg(all(
    feature = "inbound-tun",
    any(
    target_os = "windows"
    )
    ))]
        {
    Ok(Box::pin(async move {
        let fakedns = Arc::new(TokioMutex::new(FakeDns::new(fake_dns_mode)));

        for filter in fake_dns_filters.into_iter() {
            fakedns.lock().await.add_filter(filter);
        }

        let stack = NetStack::new(inbound.tag.clone(), dispatcher, nat_manager, fakedns);

            use crate::common::cmd;
            use std::process::Command;
            use std::thread;
            use crate::proxy::tun::win::{windows::Wintun, TunIpAddr};
            use std::net::Ipv4Addr;
    
            let mtu = MTU as usize;
            let tun_addr = Ipv4Addr::new(172, 0, 0, 2);
            let netmask = Ipv4Addr::new(255, 255, 255, 0);
            let tun_addr = TunIpAddr {
                ip: tun_addr,
                netmask,
            };
            // let tun = tun::create_as_async(&cfg).map_err(|e| anyhow!("create tun failed: {}", e))?;
            let gateway = cmd::get_default_ipv4_gateway().unwrap();
            println!("gateway: {:?}", gateway);

            let tun_device = Wintun::create(mtu, &[tun_addr]).unwrap();
            // let (tun_tx, tun_rx) = device.split();
            let tun_device_rx = tun_device.session.clone();
            let tun_device_tx = tun_device.session.clone();
            // let (to_tun, from_handler) = mpsc::unbounded_channel::<Box<[u8]>>();
            // let (to_tcp_handler, from_tun2) = mpsc::channel::<(Box<[u8]>, NodeId)>(CHANNEL_SIZE);
    
            // netsh interface ip set address TunMax static 240.255.0.2 255.255.255.0 11.0.68.1 3
            thread::sleep(std::time::Duration::from_millis(7));

            let out = Command::new("netsh")
                .arg("interface")
                .arg("ip")
                .arg("set")
                .arg("address")
                .arg("utun233")
                .arg("static")
                .arg("172.7.0.2")
                .arg("255.255.255.0")
                .arg("172.7.0.1")
                .arg("3")
                .status()
                .expect("failed to execute command");
            println!("process finished with: {}", out);
            let out = Command::new("route")
                .arg("add")
                .arg("1.1.1.1")
                .arg(gateway.clone())
                .arg("metric")
                .arg("5")
                .status()
                .expect("failed to execute command");
            println!("process finished with: {}", out);

            let out = Command::new("route")
                .arg("add")
                .arg("45.77.197.43")
                .arg(gateway)
                .arg("metric")
                .arg("5")
                .status()
                .expect("failed to execute command");
            println!("process finished with: {}", out);

            let s2t = tokio::task::spawn(async move {
                let mut buf = vec![0; mtu as usize];
                loop {
                    match stack_reader.read(&mut buf).await {
                        Ok(0) => {
                            debug!("read stack eof");
                            return;
                        }
    
                        Ok(n) => match tun_device_tx.allocate_send_packet(n as u16) {
                            Ok(mut packet) => {
                                packet.bytes_mut().copy_from_slice(&buf[..n]);
                                tun_device_tx.send_packet(packet);
                            }
                            Err(err) => {
                                log::error!("allocate send packet failed:{:?}", err);
                            }
                        },
                        // Ok(n) => match tun_device_tx.send_packet(&buf[..n]) {
                        //     Ok(_) => (),
                        //     Err(e) => {
                        //         warn!("send pkt to tun failed: {}", e);
                        //         return;
                        //     }
                        // },
                        Err(err) => {
                            warn!("read stack failed {:?}", err);
                            return;
                        }
                    }
                }
            });
    
            let t2s = tokio::task::spawn(async move {
                let mut packet = [0u8; MTU];
    
                loop {
                    match tun_device_rx.receive_blocking() {
                        Ok(mut packet) => {
                            packet.bytes().len();
                            match stack_writer.write(packet.bytes()).await {
                                Ok(_) => (),
                                Err(e) => {
                                    warn!("write pkt to stack failed: {}", e);
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            error!("Got error while reading: {:?}", err);
                            break;
                        }
                    }
                }
    
                // while let Ok(size) = tun_device.recv_packet(&mut packet) {
                //     if size == 0 {
                //         continue;
                //     }
                //     match stack_writer.write(&packet[..size]).await {
                //         Ok(_) => (),
                //         Err(e) => {
                //             warn!("write pkt to stack failed: {}", e);
                //             return;
                //         }
                //     }
                // }
            });
    
            info!("tun inbound started");
            futures::future::select(t2s, s2t).await;
            info!("tun inbound exited");


    }))}
}
