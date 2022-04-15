use anyhow::{anyhow, Result};
use indexmap::{map::Values, IndexMap};
use log::*;
use protobuf::Message;
use std::convert::From;
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
    // app::SyncDnsClient,
    config::{self, Outbound},
    proxy::{outbound::HandlerBuilder, *},
};

// use super::selector::OutboundSelector;

pub struct OutboundManager {
    handlers: IndexMap<String, AnyOutboundHandler>,
    // external_handlers: super::plugin::ExternalHandlers,
    // selectors: Arc<super::Selectors>,
    default_handler: Option<String>,
    // abort_handles: Vec<AbortHandle>,
}

impl OutboundManager {
    #[allow(clippy::type_complexity)]
    fn load_handlers(
        outbounds: &Vec<Outbound>,
        // dns_client: SyncDnsClient,
        handlers: &mut IndexMap<String, AnyOutboundHandler>,
        // external_handlers: &mut super::plugin::ExternalHandlers,
        default_handler: &mut Option<String>,
        // abort_handles: &mut Vec<AbortHandle>,
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
        Ok(())
    }

    // TODO make this non-async?
    pub async fn reload(
        &mut self,
        outbounds: &Vec<Outbound>,
        // dns_client: SyncDnsClient,
    ) -> Result<()> {
        /*        // Save outound select states.
                let mut selected_outbounds = IndexMap::new();
                for (k, v) in self.selectors.iter() {
                    selected_outbounds.insert(k.to_owned(), v.read().await.get_selected_tag());
                }
        */
        // Load new outbounds.
        let mut handlers: IndexMap<String, AnyOutboundHandler> = IndexMap::new();

        // let mut external_handlers = super::plugin::ExternalHandlers::new();
        let mut default_handler: Option<String> = None;
        // let mut abort_handles: Vec<AbortHandle> = Vec::new();
        // let mut selectors: super::Selectors = IndexMap::new();
        for _i in 0..4 {
            Self::load_handlers(
                outbounds,
                // dns_client.clone(),
                &mut handlers,
                // &mut external_handlers,
                &mut default_handler,
                // &mut abort_handles,
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

        /*        // Abort spawned tasks inside handlers.
        for abort_handle in self.abort_handles.iter() {
            abort_handle.abort();
        }*/

        self.handlers = handlers;
        // self.external_handlers = external_handlers;
        // self.selectors = Arc::new(selectors);
        self.default_handler = default_handler;
        // self.abort_handles = abort_handles;
        Ok(())
    }

    pub fn new(
        outbounds: &Vec<Outbound>,
        // dns_client: SyncDnsClient,
    ) -> Result<Self> {
        let mut handlers: IndexMap<String, AnyOutboundHandler> = IndexMap::new();
        // let mut external_handlers = super::plugin::ExternalHandlers::new();
        let mut default_handler: Option<String> = None;
        // let mut abort_handles: Vec<AbortHandle> = Vec::new();
        // let mut selectors: super::Selectors = IndexMap::new();
        for _i in 0..4 {
            Self::load_handlers(
                outbounds,
                // dns_client.clone(),
                &mut handlers,
                // &mut external_handlers,
                &mut default_handler,
                // &mut abort_handles,
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
            // abort_handles,
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
    inner: Values<'a, String, AnyOutboundHandler>,
}

impl<'a> Iterator for Handlers<'a> {
    type Item = &'a AnyOutboundHandler;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
