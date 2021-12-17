use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::time::Duration;

use futures::future::{self, Either};
use log::*;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::{
    app::SyncDnsClient,
    common::sniff,
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
    outbound_tag_color: Option<colored::Color>,
    handshake_time: u128,
) {
    if let Some(color) = outbound_tag_color {
        use colored::Colorize;
        let network_color = match sess.network {
            Network::Tcp => colored::Color::Blue,
            Network::Udp => colored::Color::Yellow,
        };
        info!(
            "[{}] [{}] [{}] [{}ms] {}",
            &sess.inbound_tag,
            sess.network.to_string().color(network_color),
            outbound_tag.color(color),
            handshake_time,
            &sess.destination,
        );
    } else {
        info!(
            "[{}] [{}] [{}] [{}ms] {}",
            sess.network, &sess.inbound_tag, outbound_tag, handshake_time, &sess.destination,
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
        let mut lhs: Box<dyn ProxyStream> =
            if !sess.destination.is_domain() && sess.destination.port() == 443 {
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
                                        debug!(
                                            "convert sniffed domain {} to destination failed: {}",
                                            &domain, e,
                                        );
                                        return;
                                    }
                                };
                        }
                    }
                    Err(e) => {
                        trace!(
                            "sniff tcp uplink {} -> {} failed: {}",
                            &sess.source,
                            &sess.destination,
                            e,
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
            let outbound = match router.pick_route(sess).await {
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
            };
            outbound
        };

        let h = if let Some(h) = self.outbound_manager.read().await.get(&outbound) {
            h
        } else {
            // FIXME use  the default handler
            debug!("handler not found");
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
                    return;
                }
            };
        match TcpOutboundHandler::handle(h.as_ref(), sess, stream).await {
            Ok(mut rhs) => {
                let elapsed = tokio::time::Instant::now().duration_since(handshake_start);

                if *crate::option::LOG_NO_COLOR {
                    log_request(sess, h.tag(), None, elapsed.as_millis());
                } else {
                    log_request(sess, h.tag(), Some(h.color()), elapsed.as_millis());
                }

                use super::copy::CopyFuture;
                let copy =
                    CopyFuture::with_capacity(lhs, rhs, Duration::from_secs(*option::TCP_DOWNLINK_TIMEOUT));
                let _ = copy.await;
                // tokio::io::copy_bidirectional(&mut rhs,&mut lhs).await;
            }
            Err(e) => {
                debug!(
                    "dispatch tcp {} -> {} to [{}] failed: {}",
                    &sess.source,
                    &sess.destination,
                    &h.tag(),
                    e
                );

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
            let outbound = match router.pick_route(sess).await {
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
                        return Err(io::Error::new(ErrorKind::Other, "no available handler"));
                    }
                }
            };
            outbound
        };

        let h = if let Some(h) = self.outbound_manager.read().await.get(&outbound) {
            h
        } else {
            return Err(io::Error::new(ErrorKind::Other, "handler not found"));
        };

        let handshake_start = tokio::time::Instant::now();
        let transport =
            crate::proxy::connect_udp_outbound(sess, self.dns_client.clone(), &h).await?;
        match UdpOutboundHandler::handle(h.as_ref(), sess, transport).await {
            Ok(c) => {
                let elapsed = tokio::time::Instant::now().duration_since(handshake_start);

                if *crate::option::LOG_NO_COLOR {
                    log_request(sess, h.tag(), None, elapsed.as_millis());
                } else {
                    log_request(sess, h.tag(), Some(h.color()), elapsed.as_millis());
                }

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
                Err(e)
            }
        }
    }
}
