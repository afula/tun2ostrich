use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::time::Duration;

use log::*;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::{
    app::SyncDnsClient,
    common::{self, sniff},
    option,
    proxy::{OutboundDatagram, ProxyStream, TcpOutboundHandler, UdpOutboundHandler},
    session::{Network, Session, SocksAddr},
};

use super::outbound::manager::OutboundManager;
use super::router::Router;

#[inline]
fn log_request(
    sess: &Session,
    outbound_tag: &str,
    outbound_tag_color: colored::Color,
    handshake_time: Option<u128>,
) {
    let hs = handshake_time.map_or("failed".to_string(), |hs| format!("{}ms", hs));
    if !*crate::option::LOG_NO_COLOR {
        use colored::Colorize;
        let network_color = match sess.network {
            Network::Tcp => colored::Color::Blue,
            Network::Udp => colored::Color::Yellow,
        };
        info!(
            "[{}] [{}] [{}] [{}] {}",
            &sess.inbound_tag,
            sess.network.to_string().color(network_color),
            outbound_tag.color(outbound_tag_color),
            hs,
            &sess.destination,
        );
    } else {
        info!(
            "[{}] [{}] [{}] [{}] {}",
            sess.network, &sess.inbound_tag, outbound_tag, hs, &sess.destination,
        );
    }
}

pub struct Dispatcher {
    outbound_manager: Arc<RwLock<OutboundManager>>,
    router: Arc<RwLock<Router>>,
    dns_client: SyncDnsClient,
}

impl Dispatcher {
    pub fn new(
        outbound_manager: Arc<RwLock<OutboundManager>>,
        router: Arc<RwLock<Router>>,
        dns_client: SyncDnsClient,
    ) -> Self {
        Dispatcher {
            outbound_manager,
            router,
            dns_client,
        }
    }

    pub async fn dispatch_tcp<T>(&self, sess: &mut Session, lhs: T)
    where
        T: 'static + AsyncRead + AsyncWrite + Unpin + Send + Sync,
    {
        let mut lhs: Box<dyn ProxyStream> = if *option::DOMAIN_SNIFFING
            && !sess.destination.is_domain()
            && sess.destination.port() == 443
        {
            let mut lhs = sniff::SniffingStream::new(lhs);
            match lhs.sniff().await {
                Ok(res) => {
                    if let Some(domain) = res {
                        debug!(
                            "sniffed domain {} for tcp link {} <-> {}",
                            &domain, &sess.source, &sess.destination,
                        );
                        sess.destination =
                            match SocksAddr::try_from((&domain, sess.destination.port())) {
                                Ok(a) => a,
                                Err(e) => {
                                    warn!(
                                        "convert sniffed domain {} to destination failed: {}",
                                        &domain, e,
                                    );
                                    return;
                                }
                            };
                    }
                }
                Err(e) => {
                    debug!(
                        "sniff tcp uplink {} -> {} failed: {}",
                        &sess.source, &sess.destination, e,
                    );
                    return;
                }
            }
            Box::new(lhs)
        } else {
            Box::new(lhs)
        };

        let outbound = {
            let router = self.router.read().await;
            match router.pick_route(sess).await {
                Ok(tag) => {
                    debug!(
                        "picked route [{}] for {} -> {}",
                        tag, &sess.source, &sess.destination
                    );
                    tag.to_owned()
                }
                Err(err) => {
                    trace!("pick route failed: {}", err);
                    if let Some(tag) = self.outbound_manager.read().await.default_handler() {
                        debug!(
                            "picked default route [{}] for {} -> {}",
                            tag, &sess.source, &sess.destination
                        );
                        tag
                    } else {
                        warn!("can not find any handlers");
                        if let Err(e) = lhs.shutdown().await {
                            debug!(
                                "tcp downlink {} <- {} error: {}",
                                &sess.source, &sess.destination, e,
                            );
                        }
                        return;
                    }
                }
            }
        };

        let h = if let Some(h) = self.outbound_manager.read().await.get(&outbound) {
            h
        } else {
            // FIXME use  the default handler
            warn!("handler not found");
            if let Err(e) = lhs.shutdown().await {
                debug!(
                    "tcp downlink {} <- {} error: {}",
                    &sess.source, &sess.destination, e,
                );
            }
            return;
        };

        let handshake_start = tokio::time::Instant::now();
        let stream =
            match crate::proxy::connect_tcp_outbound(sess, self.dns_client.clone(), &h).await {
                Ok(s) => s,
                Err(e) => {
                    debug!(
                        "dispatch tcp {} -> {} to [{}] failed: {}",
                        &sess.source,
                        &sess.destination,
                        &h.tag(),
                        e
                    );
                    log_request(sess, h.tag(), h.color(), None);
                    return;
                }
            };
        match TcpOutboundHandler::handle(h.as_ref(), sess, stream).await {
            Ok(mut rhs) => {
                let elapsed = tokio::time::Instant::now().duration_since(handshake_start);

                log_request(sess, h.tag(), h.color(), Some(elapsed.as_millis()));

                match common::io::copy_buf_bidirectional_with_timeout(
                    &mut lhs,
                    &mut rhs,
                    *option::LINK_BUFFER_SIZE * 1024,
                    Duration::from_secs(*option::TCP_UPLINK_TIMEOUT),
                    Duration::from_secs(*option::TCP_DOWNLINK_TIMEOUT),
                )
                .await
                {
                    Ok((up_count, down_count)) => {
                        debug!(
                            "tcp link {} <-> {} done, ({}, {}) bytes transfered [{}]",
                            &sess.source,
                            &sess.destination,
                            up_count,
                            down_count,
                            &h.tag(),
                        );
                    }
                    Err(e) => {
                        debug!(
                            "tcp link {} <-> {} error: {} [{}]",
                            &sess.source,
                            &sess.destination,
                            e,
                            &h.tag()
                        );
                    }
                }
            }
            Err(e) => {
                debug!(
                    "dispatch tcp {} -> {} to [{}] failed: {}",
                    &sess.source,
                    &sess.destination,
                    &h.tag(),
                    e
                );

                log_request(sess, h.tag(), h.color(), None);

                if let Err(e) = lhs.shutdown().await {
                    debug!(
                        "tcp downlink {} <- {} error: {} [{}]",
                        &sess.source,
                        &sess.destination,
                        e,
                        &h.tag()
                    );
                }
            }
        }
    }

    pub async fn dispatch_udp(&self, sess: &Session) -> io::Result<Box<dyn OutboundDatagram>> {
        let outbound = {
            let router = self.router.read().await;
            match router.pick_route(sess).await {
                Ok(tag) => {
                    debug!(
                        "picked route [{}] for {} -> {}",
                        tag, &sess.source, &sess.destination
                    );
                    tag.to_owned()
                }
                Err(err) => {
                    trace!("pick route failed: {}", err);
                    if let Some(tag) = self.outbound_manager.read().await.default_handler() {
                        debug!(
                            "picked default route [{}] for {} -> {}",
                            tag, &sess.source, &sess.destination
                        );
                        tag
                    } else {
                        warn!("no handler found");
                        return Err(io::Error::new(ErrorKind::Other, "no available handler"));
                    }
                }
            }
        };

        let h = if let Some(h) = self.outbound_manager.read().await.get(&outbound) {
            h
        } else {
            warn!("handler not found");
            return Err(io::Error::new(ErrorKind::Other, "handler not found"));
        };

        let handshake_start = tokio::time::Instant::now();
        let transport =
            crate::proxy::connect_udp_outbound(sess, self.dns_client.clone(), &h).await?;
        match UdpOutboundHandler::handle(h.as_ref(), sess, transport).await {
            Ok(c) => {
                let elapsed = tokio::time::Instant::now().duration_since(handshake_start);

                log_request(sess, h.tag(), h.color(), Some(elapsed.as_millis()));

                Ok(c)
            }
            Err(e) => {
                debug!(
                    "dispatch udp {} -> {} to [{}] failed: {}",
                    &sess.source,
                    &sess.destination,
                    &h.tag(),
                    e
                );
                log_request(sess, h.tag(), h.color(), None);
                Err(e)
            }
        }
    }
}
