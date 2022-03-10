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
        target_os = "windows"
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
            target_os = "windows"
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
                target_os = "windows"
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
                        target_os = "windows"
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
                    target_os = "windows"
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
            target_os = "windows"
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
            target_os = "windows"
        )
    ))]
    pub fn has_tun_listener(&self) -> bool {
        self.tun_listener.is_some()
    }

    pub fn tun_auto(&self) -> bool {
        self.tun_auto
    }
}
