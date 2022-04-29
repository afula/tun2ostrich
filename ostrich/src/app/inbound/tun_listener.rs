use std::sync::Arc;

use anyhow::Result;

use crate::app::dispatcher::Dispatcher;
use crate::app::nat_manager::NatManager;
use crate::config::{Config, Inbound};
use crate::proxy::tun;
use crate::Runner;

pub struct TunInboundListener {
    pub inbound: Inbound,
    pub dispatcher: Arc<Dispatcher>,
    pub nat_manager: Arc<NatManager>,
    #[cfg(target_os = "windows")]
    pub wintun_path: String,
}

impl TunInboundListener {
    pub fn listen(&self) -> Result<Runner> {
        tun::inbound::new(
            self.inbound.clone(),
            self.dispatcher.clone(),
            self.nat_manager.clone(),
            #[cfg(target_os = "windows")]
            self.wintun_path.clone(),
        )
    }
}
