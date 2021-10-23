use std::{
    collections::{hash_map, HashMap},
    convert::From,
    sync::atomic::AtomicUsize,
    sync::Arc,
};

use anyhow::{anyhow, Result};
use futures::future::AbortHandle;
use log::*;
use protobuf::Message;
use tokio::sync::RwLock;

use crate::proxy::null;

/*#[cfg(feature = "outbound-chain")]
use crate::proxy::chain;
#[cfg(feature = "outbound-failover")]
use crate::proxy::failover;

#[cfg(feature = "outbound-retry")]
use crate::proxy::retry;*/

/*#[cfg(feature = "outbound-select")]
use crate::proxy::select;
#[cfg(feature = "outbound-tryall")]
use crate::proxy::tryall;*/

#[cfg(feature = "outbound-direct")]
use crate::proxy::direct;
/*#[cfg(feature = "outbound-drop")]
use crate::proxy::drop;*/

#[cfg(feature = "outbound-trojan")]
use crate::proxy::trojan;

use crate::proxy::trojan::outbound::tls::make_config;
use crate::{
    app::SyncDnsClient,
    config::{self, Outbound},
    proxy::{self, outbound::HandlerBuilder, *},
};

// use super::selector::OutboundSelector;

pub struct OutboundManager {
    handlers: HashMap<String, AnyOutboundHandler>,
    // external_handlers: super::plugin::ExternalHandlers,
    // selectors: Arc<super::Selectors>,
    default_handler: Option<String>,
    abort_handles: Vec<AbortHandle>,
}

impl OutboundManager {
    #[allow(clippy::type_complexity)]
    fn load_handlers(
        outbounds: &protobuf::RepeatedField<Outbound>,
        dns_client: SyncDnsClient,
        handlers: &mut HashMap<String, AnyOutboundHandler>,
        // external_handlers: &mut super::plugin::ExternalHandlers,
        default_handler: &mut Option<String>,
        abort_handles: &mut Vec<AbortHandle>,
    ) -> Result<()> {
        for outbound in outbounds.iter() {
            let tag = String::from(&outbound.tag);
            if handlers.contains_key(&tag) {
                continue;
            }
            if default_handler.is_none() {
                default_handler.replace(String::from(&outbound.tag));
                debug!("default handler [{}]", &outbound.tag);
            }
            match outbound.protocol.as_str() {
                #[cfg(feature = "outbound-direct")]
                "direct" => {
                    handlers.insert(
                        tag.clone(),
                        HandlerBuilder::default()
                            .tag(tag.clone())
                            .color(colored::Color::Green)
                            .tcp_handler(Box::new(direct::TcpHandler))
                            .udp_handler(Box::new(direct::UdpHandler))
                            .build(),
                    );
                    trace!("added handler [{}]", &tag);
                }
                #[cfg(feature = "outbound-trojan")]
                "trojan" => {
                    let settings =
                        config::TrojanOutboundSettings::parse_from_bytes(&outbound.settings)
                            .map_err(|e| anyhow!("invalid [{}] outbound settings: {}", &tag, e))?;
                    let server_name = settings.server_name.clone();

                    let tls_config = make_config(&settings);

                    let tcp = Box::new(trojan::outbound::TcpHandler {
                        address: settings.address.clone(),
                        port: settings.port as u16,
                        password: settings.password.clone(),

                        server_name: server_name.clone(),
                        tls_config: tls_config.clone(),
                    });
                    let udp = Box::new(trojan::outbound::UdpHandler {
                        address: settings.address,
                        port: settings.port as u16,
                        password: settings.password,

                        server_name: server_name.clone(),
                        tls_config: tls_config.clone(),
                    });
                    let handler = HandlerBuilder::default()
                        .tag(tag.clone())
                        .tcp_handler(tcp)
                        .udp_handler(udp)
                        .build();
                    handlers.insert(tag.clone(), handler);
                    trace!("added handler [{}]", &tag);
                }
                _ => continue,
            }
        }

        // FIXME a better way to find outbound deps?
        /*        for _i in 0..8 {
            'outbounds: for outbound in outbounds.iter() {
                let tag = String::from(&outbound.tag);
                if handlers.contains_key(&tag) {
                    continue;
                }
                match outbound.protocol.as_str() {
                    #[cfg(feature = "outbound-tryall")]
                    "tryall" => {
                        let settings =
                            config::TryAllOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        let mut actors = Vec::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            } else {
                                continue 'outbounds;
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }
                        let tcp = Box::new(tryall::TcpHandler {
                            actors: actors.clone(),
                            delay_base: settings.delay_base,
                            dns_client: dns_client.clone(),
                        });
                        let udp = Box::new(tryall::UdpHandler {
                            actors,
                            delay_base: settings.delay_base,
                            dns_client: dns_client.clone(),
                        });
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(tcp)
                            .udp_handler(udp)
                            .build();
                        handlers.insert(tag.clone(), handler);
                        trace!(
                            "added handler [{}] with actors: {}",
                            &tag,
                            settings.actors.join(",")
                        );
                    }
                    #[cfg(feature = "outbound-failover")]
                    "failover" => {
                        let settings =
                            config::FailOverOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        let mut actors = Vec::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            } else {
                                continue 'outbounds;
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }
                        let (tcp, mut tcp_abort_handles) = failover::TcpHandler::new(
                            actors.clone(),
                            settings.fail_timeout,
                            settings.health_check,
                            settings.check_interval,
                            settings.failover,
                            settings.fallback_cache,
                            settings.cache_size as usize,
                            settings.cache_timeout as u64,
                            dns_client.clone(),
                        );
                        let (udp, mut udp_abort_handles) = failover::UdpHandler::new(
                            actors,
                            settings.fail_timeout,
                            settings.health_check,
                            settings.check_interval,
                            settings.failover,
                            dns_client.clone(),
                        );
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(Box::new(tcp))
                            .udp_handler(Box::new(udp))
                            .build();
                        handlers.insert(tag.clone(), handler);
                        abort_handles.append(&mut tcp_abort_handles);
                        abort_handles.append(&mut udp_abort_handles);
                        trace!(
                            "added handler [{}] with actors: {}",
                            &tag,
                            settings.actors.join(",")
                        );
                    }
                    #[cfg(feature = "outbound-chain")]
                    "chain" => {
                        let settings =
                            config::ChainOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        let mut actors = Vec::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            } else {
                                continue 'outbounds;
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }
                        let tcp = Box::new(chain::outbound::TcpHandler {
                            actors: actors.clone(),
                        });
                        let udp = Box::new(chain::outbound::UdpHandler {
                            actors: actors.clone(),
                        });
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(tcp)
                            .udp_handler(udp)
                            .build();
                        handlers.insert(tag.clone(), handler);
                        trace!(
                            "added handler [{}] with actors: {}",
                            &tag,
                            settings.actors.join(",")
                        );
                    }
                    #[cfg(feature = "outbound-retry")]
                    "retry" => {
                        let settings =
                            config::RetryOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        let mut actors = Vec::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.push(a.clone());
                            } else {
                                continue 'outbounds;
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }
                        let tcp = Box::new(retry::TcpHandler {
                            actors: actors.clone(),
                            attempts: settings.attempts as usize,
                            dns_client: dns_client.clone(),
                        });
                        let udp = Box::new(retry::UdpHandler {
                            actors,
                            attempts: settings.attempts as usize,
                            dns_client: dns_client.clone(),
                        });
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(tcp)
                            .udp_handler(udp)
                            .build();
                        handlers.insert(tag.clone(), handler);
                        trace!(
                            "added handler [{}] with actors: {}",
                            &tag,
                            settings.actors.join(",")
                        );
                    }
                    "plugin" => {
                        let settings =
                            config::PluginOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        unsafe {
                            external_handlers
                                .new_handler(settings.path, &tag, &settings.args)
                                .unwrap()
                        };
                        let tcp = Box::new(super::plugin::ExternalTcpOutboundHandlerProxy(
                            external_handlers.get_tcp_handler(&tag).unwrap(),
                        ));
                        let udp = Box::new(super::plugin::ExternalUdpOutboundHandlerProxy(
                            external_handlers.get_udp_handler(&tag).unwrap(),
                        ));
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(tcp)
                            .udp_handler(udp)
                            .build();
                        handlers.insert(tag.clone(), handler);
                        trace!("added handler [{}]", &tag,);
                    }
                    _ => continue,
                }
            }
        }*/

        Ok(())
    }

    /*    fn load_selectors(
        outbounds: &protobuf::RepeatedField<Outbound>,
        handlers: &mut HashMap<String, AnyOutboundHandler>,
        external_handlers: &mut super::plugin::ExternalHandlers,
        selectors: &mut super::Selectors,
    ) -> Result<()> {
        // FIXME a better way to find outbound deps?
        for _i in 0..8 {
            'outbounds: for outbound in outbounds.iter() {
                let tag = String::from(&outbound.tag);
                if handlers.contains_key(&tag) || selectors.contains_key(&tag) {
                    continue;
                }
                #[allow(clippy::single_match)]
                match outbound.protocol.as_str() {
                    #[cfg(feature = "outbound-select")]
                    "select" => {
                        let settings =
                            config::SelectOutboundSettings::parse_from_bytes(&outbound.settings)
                                .map_err(|e| {
                                    anyhow!("invalid [{}] outbound settings: {}", &tag, e)
                                })?;
                        let mut actors = HashMap::new();
                        for actor in settings.actors.iter() {
                            if let Some(a) = handlers.get(actor) {
                                actors.insert(actor.to_owned(), a.clone());
                            } else {
                                continue 'outbounds;
                            }
                        }
                        if actors.is_empty() {
                            continue;
                        }

                        let mut selector = OutboundSelector::new(tag.clone(), actors);
                        if let Ok(Some(selected)) = super::selector::get_selected_from_cache(&tag) {
                            // FIXME handle error
                            let _ = selector.set_selected(&selected);
                        } else {
                            let _ = selector.set_selected(&settings.actors[0]);
                        }
                        let selector = Arc::new(RwLock::new(selector));

                        let tcp = Box::new(select::TcpHandler {
                            selector: selector.clone(),
                        });
                        let udp = Box::new(select::UdpHandler {
                            selector: selector.clone(),
                        });
                        selectors.insert(tag.clone(), selector);
                        let handler = HandlerBuilder::default()
                            .tag(tag.clone())
                            .tcp_handler(tcp)
                            .udp_handler(udp)
                            .build();
                        handlers.insert(tag.clone(), handler);
                        trace!(
                            "added handler [{}] with actors: {}",
                            &tag,
                            settings.actors.join(",")
                        );
                    }
                    _ => continue,
                }
            }
        }

        Ok(())
    }*/

    // TODO make this non-async?
    pub async fn reload(
        &mut self,
        outbounds: &protobuf::RepeatedField<Outbound>,
        dns_client: SyncDnsClient,
    ) -> Result<()> {
        /*        // Save outound select states.
                let mut selected_outbounds = HashMap::new();
                for (k, v) in self.selectors.iter() {
                    selected_outbounds.insert(k.to_owned(), v.read().await.get_selected_tag());
                }
        */
        // Load new outbounds.
        let mut handlers: HashMap<String, AnyOutboundHandler> = HashMap::new();

        // let mut external_handlers = super::plugin::ExternalHandlers::new();
        let mut default_handler: Option<String> = None;
        let mut abort_handles: Vec<AbortHandle> = Vec::new();
        // let mut selectors: super::Selectors = HashMap::new();
        for _i in 0..4 {
            Self::load_handlers(
                outbounds,
                dns_client.clone(),
                &mut handlers,
                // &mut external_handlers,
                &mut default_handler,
                &mut abort_handles,
            )?;
            /*            Self::load_selectors(
                outbounds,
                &mut handlers,
                &mut external_handlers,
                &mut selectors,
            )?;*/
        }

        /*        // Restore outbound select states.
        for (k, v) in selected_outbounds.iter() {
            for (k2, v2) in selectors.iter_mut() {
                if k == k2 {
                    if let Some(v) = v {
                        let _ = v2.write().await.set_selected(v);
                    }
                }
            }
        }*/

        // Abort spawned tasks inside handlers.
        for abort_handle in self.abort_handles.iter() {
            abort_handle.abort();
        }

        self.handlers = handlers;
        // self.external_handlers = external_handlers;
        // self.selectors = Arc::new(selectors);
        self.default_handler = default_handler;
        self.abort_handles = abort_handles;
        Ok(())
    }

    pub fn new(
        outbounds: &protobuf::RepeatedField<Outbound>,
        dns_client: SyncDnsClient,
    ) -> Result<Self> {
        let mut handlers: HashMap<String, AnyOutboundHandler> = HashMap::new();
        // let mut external_handlers = super::plugin::ExternalHandlers::new();
        let mut default_handler: Option<String> = None;
        let mut abort_handles: Vec<AbortHandle> = Vec::new();
        // let mut selectors: super::Selectors = HashMap::new();
        for _i in 0..4 {
            Self::load_handlers(
                outbounds,
                dns_client.clone(),
                &mut handlers,
                // &mut external_handlers,
                &mut default_handler,
                &mut abort_handles,
            )?;
            /*            Self::load_selectors(
                outbounds,
                &mut handlers,
                &mut external_handlers,
                &mut selectors,
            )?;*/
        }
        Ok(OutboundManager {
            handlers,
            // external_handlers,
            // selectors: Arc::new(selectors),
            default_handler,
            abort_handles,
        })
    }

    pub fn add(&mut self, tag: String, handler: AnyOutboundHandler) {
        self.handlers.insert(tag, handler);
    }

    pub fn get(&self, tag: &str) -> Option<AnyOutboundHandler> {
        self.handlers.get(tag).map(Clone::clone)
    }

    pub fn default_handler(&self) -> Option<String> {
        self.default_handler.as_ref().map(Clone::clone)
    }

    pub fn handlers(&self) -> Handlers {
        Handlers {
            inner: self.handlers.values(),
        }
    }

    /*    pub fn get_selector(&self, tag: &str) -> Option<Arc<RwLock<OutboundSelector>>> {
        self.selectors.get(tag).map(Clone::clone)
    }*/
}

pub struct Handlers<'a> {
    inner: hash_map::Values<'a, String, AnyOutboundHandler>,
}

impl<'a> Iterator for Handlers<'a> {
    type Item = &'a AnyOutboundHandler;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
