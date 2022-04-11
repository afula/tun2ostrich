use indexmap::IndexMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use protobuf::Message;

use crate::app::dispatcher::Dispatcher;
use crate::app::nat_manager::NatManager;
use crate::config;
use crate::proxy;
use crate::proxy::AnyInboundHandler;
use crate::Runner;

#[cfg(feature = "inbound-socks")]
use crate::proxy::socks;

use super::network_listener::NetworkInboundListener;

#[cfg(all(
    feature = "inbound-tun",
    any(
        target_os = "ios",
        target_os = "android",
        target_os = "macos",
        target_os = "linux",
        // target_os = "windows"
    )
))]
use super::tun_listener::TunInboundListener;

pub struct InboundManager {
    network_listeners: IndexMap<String, NetworkInboundListener>,
    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            // target_os = "windows"
        )
    ))]
    tun_listener: Option<TunInboundListener>,
    tun_auto: bool,
}

impl InboundManager {
    pub fn new(
        inbounds: &protobuf::RepeatedField<config::Inbound>,
        dispatcher: Arc<Dispatcher>,
        nat_manager: Arc<NatManager>,
    ) -> Result<Self> {
        let mut handlers: IndexMap<String, AnyInboundHandler> = IndexMap::new();

        for inbound in inbounds.iter() {
            let tag = String::from(&inbound.tag);
            match inbound.protocol.as_str() {
                #[cfg(feature = "inbound-socks")]
                "socks" => {
                    let tcp = Arc::new(socks::inbound::TcpHandler);
                    let udp = Arc::new(socks::inbound::UdpHandler);
                    let handler = Arc::new(proxy::inbound::Handler::new(
                        tag.clone(),
                        Some(tcp),
                        Some(udp),
                    ));
                    handlers.insert(tag.clone(), handler);
                    #[cfg(all(feature = "inbound-tun", any(target_os = "windows")))]
                    {
                        use crate::common::cmd;
                        use std::process::Command;
                        use tokio::sync::mpsc;
                        let (tun_tx, mut tun_rx) = mpsc::channel(1);

                        tokio::spawn(async move {
                            let _ = Command::new("misc/tun2socks")
                                .arg("-device")
                                .arg("tun://utun233")
                                .arg("-proxy")
                                .arg("socks5://127.0.0.1:1086")
                                // flag.StringVar(&key.LogLevel, "loglevel", "info", "Log level [debug|info|warning|error|silent]")
                                .arg("-loglevel")
                                .arg("warning")
                                .spawn()
                                .expect("failed to execute process");
                            println!("init tun device process finished");
                            if let Err(e) = tun_tx.send(()).await {
                                log::warn!("tun device completed signal failed: {}", e);
                            }
                        });

                        tokio::spawn(async move {
                            let _ = tun_rx.recv().await;
                            std::thread::sleep(std::time::Duration::from_secs(7));

                            let gateway = cmd::get_default_ipv4_gateway().unwrap();
                            println!("gateway: {:?}", gateway);

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
                        });
                    }
                }
                _ => (),
            }
        }

        let mut network_listeners: IndexMap<String, NetworkInboundListener> = IndexMap::new();

        #[cfg(all(
            feature = "inbound-tun",
            any(
                target_os = "ios",
                target_os = "android",
                target_os = "macos",
                target_os = "linux",
                // target_os = "windows"
            )
        ))]
        let mut tun_listener: Option<TunInboundListener> = None;

        let mut tun_auto = false;

        for inbound in inbounds.iter() {
            let tag = String::from(&inbound.tag);
            match inbound.protocol.as_str() {
                #[cfg(all(
                    feature = "inbound-tun",
                    any(
                        target_os = "ios",
                        target_os = "android",
                        target_os = "macos",
                        target_os = "linux",
                        // target_os = "windows"
                    )
                ))]
                "tun" => {
                    let listener = TunInboundListener {
                        inbound: inbound.clone(),
                        dispatcher: dispatcher.clone(),
                        nat_manager: nat_manager.clone(),
                    };
                    tun_listener.replace(listener);
                    let settings =
                        crate::config::TunInboundSettings::parse_from_bytes(&inbound.settings)?;
                    tun_auto = settings.auto;
                }
                _ => {
                    if inbound.port != 0 {
                        if let Some(h) = handlers.get(&tag) {
                            let listener = NetworkInboundListener {
                                address: inbound.address.clone(),
                                port: inbound.port as u16,
                                handler: h.clone(),
                                dispatcher: dispatcher.clone(),
                                nat_manager: nat_manager.clone(),
                            };
                            network_listeners.insert(tag.clone(), listener);
                        }
                    }
                }
            }
        }

        Ok(InboundManager {
            network_listeners,
            #[cfg(all(
                feature = "inbound-tun",
                any(
                    target_os = "ios",
                    target_os = "android",
                    target_os = "macos",
                    target_os = "linux",
                    // target_os = "windows"
                )
            ))]
            tun_listener,
            tun_auto,
        })
    }

    pub fn get_network_runners(&self) -> Result<Vec<Runner>> {
        let mut runners: Vec<Runner> = Vec::new();
        for (_, listener) in self.network_listeners.iter() {
            runners.append(&mut listener.listen()?);
        }
        Ok(runners)
    }

    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            // target_os = "windows"
        )
    ))]
    pub fn get_tun_runner(&self) -> Result<Runner> {
        if let Some(listener) = &self.tun_listener {
            return listener.listen();
        }
        Err(anyhow!("no tun inbound"))
    }

    #[cfg(all(
        feature = "inbound-tun",
        any(
            target_os = "ios",
            target_os = "android",
            target_os = "macos",
            target_os = "linux",
            // target_os = "windows"
        )
    ))]
    pub fn has_tun_listener(&self) -> bool {
        self.tun_listener.is_some()
    }

    pub fn tun_auto(&self) -> bool {
        self.tun_auto
    }
}
