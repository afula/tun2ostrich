#![feature(once_cell)]

#[cfg(feature = "api")]
use crate::app::api::api_server::ApiServer;
use anyhow::anyhow;
use app::{
    dispatcher::Dispatcher, dns_client::DnsClient, inbound::manager::InboundManager,
    nat_manager::NatManager, outbound::manager::OutboundManager, router::Router,
};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use protobuf::Enum;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{interval_at, Instant};

pub mod app;
pub mod common;
pub mod config;
#[cfg(any(target_os = "ios", target_os = "macos", target_os = "android"))]
pub mod mobile;
pub mod option;
pub mod proxy;
pub mod session;
#[cfg(all(
    feature = "inbound-tun",
    any(target_os = "macos", target_os = "linux", target_os = "windows")
))]
mod sys;
pub mod util;

/* // #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows",target_os = "ios"))]
#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc; */

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] anyhow::Error),
    #[error("no associated config file")]
    NoConfigFile,
    #[error(transparent)]
    Io(#[from] io::Error),
    /*    #[cfg(feature = "auto-reload")]
    #[error(transparent)]
    Watcher(#[from] NotifyError),*/
    #[error(transparent)]
    AsyncChannelSend(
        #[from] tokio::sync::mpsc::error::SendError<std::sync::mpsc::SyncSender<Result<(), Error>>>,
    ),
    #[error(transparent)]
    SyncChannelRecv(#[from] std::sync::mpsc::RecvError),
    #[error("runtime manager error")]
    RuntimeManager,
}

pub type Runner = futures::future::BoxFuture<'static, ()>;

pub struct RuntimeManager {
    // #[cfg(feature = "auto-reload")]
    // rt_id: RuntimeId,
    // config_path: Option<String>,
    shutdown_tx: mpsc::Sender<()>,
    // router: Arc<RwLock<Router>>,
    // dns_client: Arc<RwLock<DnsClient>>,
    // outbound_manager: Arc<RwLock<OutboundManager>>,
}

impl RuntimeManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        // #[cfg(feature = "auto-reload")] rt_id: RuntimeId,
        // config_path: Option<String>,
        shutdown_tx: mpsc::Sender<()>,
        // router: Arc<RwLock<Router>>,
        // dns_client: Arc<RwLock<DnsClient>>,
        // outbound_manager: Arc<RwLock<OutboundManager>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            // #[cfg(feature = "auto-reload")]
            // rt_id,
            // config_path,
            shutdown_tx,
            // router,
            // dns_client,
            // outbound_manager,
        })
    }

    /*    pub async fn set_outbound_selected(&self, outbound: &str, select: &str) -> Result<(), Error> {
        if let Some(selector) = self.outbound_manager.read().await.get_selector(outbound) {
            selector
                .write()
                .await
                .set_selected(select)
                .map_err(Error::Config)
        } else {
            Err(Error::Config(anyhow!("selector not found")))
        }
    }

    pub async fn get_outbound_selected(&self, outbound: &str) -> Result<String, Error> {
        if let Some(selector) = self.outbound_manager.read().await.get_selector(outbound) {
            if let Some(tag) = selector.read().await.get_selected_tag() {
                return Ok(tag);
            }
        }
        Err(Error::Config(anyhow!("not found")))
    }*/

    /*    // This function could block by an in-progress connection dialing.
    //
    // TODO Reload FakeDns. And perhaps the inbounds as long as the listening
    // addresses haven't changed.
    pub async fn reload(&self) -> Result<(), Error> {
        let config_path = if let Some(p) = self.config_path.as_ref() {
            p
        } else {
            return Err(Error::NoConfigFile);
        };
        log::info!("reloading from config file: {}", config_path);
        let mut config = config::from_file(config_path).map_err(Error::Config)?;
        self.router.write().await.reload(&mut config.router)?;
        self.dns_client.write().await.reload(&config.dns)?;
        self.outbound_manager
            .write()
            .await
            .reload(&config.outbounds, self.dns_client.clone())
            .await?;
        log::info!("reloaded from config file: {}", config_path);
        Ok(())
    }*/

    /*    pub fn blocking_reload(&self) -> Result<(), Error> {
        let tx = self.reload_tx.clone();
        let (res_tx, res_rx) = sync_channel(0);
        if let Err(e) = tx.blocking_send(res_tx) {
            return Err(Error::AsyncChannelSend(e));
        }
        match res_rx.recv() {
            Ok(res) => res,
            Err(e) => Err(Error::SyncChannelRecv(e)),
        }
    }*/

    pub async fn shutdown(&self) -> bool {
        let tx = self.shutdown_tx.clone();
        if let Err(e) = tx.send(()).await {
            log::warn!("sending shutdown signal failed: {}", e);
            return false;
        }
        true
    }

    pub fn blocking_shutdown(&self) -> bool {
        let tx = self.shutdown_tx.clone();
        if let Err(e) = tx.blocking_send(()) {
            log::warn!("sending shutdown signal failed: {}", e);
            return false;
        }
        true
    }
}

pub type RuntimeId = u16;
const INSTANCE_ID: RuntimeId = 1;
lazy_static! {
    pub static ref RUNTIME_MANAGER: Mutex<IndexMap<RuntimeId, Arc<RuntimeManager>>> =
        Mutex::new(IndexMap::new());
}

/*pub fn reload(key: RuntimeId) -> Result<(), Error> {
    if let Ok(g) = RUNTIME_MANAGER.lock() {
        if let Some(m) = g.get(&key) {
            return m.blocking_reload();
        }
    }
    Err(Error::RuntimeManager)
}*/

pub fn shutdown() -> bool {
    if let Ok(g) = RUNTIME_MANAGER.lock() {
        if let Some(m) = g.get(&INSTANCE_ID) {
            return m.blocking_shutdown();
        }
    }
    false
}

pub fn is_running() -> bool {
    RUNTIME_MANAGER.lock().unwrap().contains_key(&INSTANCE_ID)
}

pub fn test_config(config_path: &str) -> Result<(), Error> {
    config::from_file(config_path)
        .map(|_| ())
        .map_err(Error::Config)
}

fn new_runtime() -> Result<tokio::runtime::Runtime, Error> {
    tokio::runtime::Builder::new_multi_thread()
        // .thread_stack_size(*stack_size)
        .enable_all()
        .build()
        .map_err(Error::Io)
}

#[derive(Debug)]
pub enum RuntimeOption {
    // Single-threaded runtime.
    SingleThread,
    // Multi-threaded runtime with thread stack size.
    MultiThreadAuto(usize),
    // Multi-threaded runtime with the number of worker threads and thread stack size.
    MultiThread(usize, usize),
}

#[derive(Debug)]
pub enum Config {
    File(String),
    Str(String),
    Internal(config::Config),
}

#[derive(Debug)]
pub struct StartOptions {
    // The path of the config.
    pub config: Config,
    #[cfg(target_os = "android")]
    pub socket_protect_path: Option<String>,
}

pub fn start(opts: StartOptions) -> Result<(), Error> {
    println!("start with options:\n{:#?}", opts);

    // let (reload_tx, mut reload_rx) = mpsc::channel(1);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    /*    let config_path = match opts.config {
        Config::File(ref p) => Some(p.to_owned()),
        _ => None,
    };*/

    let mut config = match opts.config {
        Config::File(p) => config::from_file(&p).map_err(Error::Config)?,
        Config::Str(s) => config::from_string(&s).map_err(Error::Config)?,
        Config::Internal(c) => c,
    };

    // FIXME Unfortunately fern does not allow re-initializing the logger,
    // should consider another logging lib if the situation doesn't change.
    let log = config
        .log
        .as_ref()
        .ok_or_else(|| Error::Config(anyhow!("empty log setting")))?;
    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        app::logger::setup_logger(log).expect("setup logger failed");
    });

    let rt = new_runtime()?;
    let _g = rt.enter();
    #[cfg(target_os = "android")]
    if let Some(p) = opts.socket_protect_path.as_ref() {
        std::env::set_var("SOCKET_PROTECT_PATH", p)
    }
    let mut tasks: Vec<Runner> = Vec::new();
    let mut runners = Vec::new();

    let dns_client = Arc::new(DnsClient::new(&config.dns).map_err(Error::Config)?);
    let outbound_manager = Arc::new(RwLock::new(
        OutboundManager::new(
            &config.outbounds, // dns_client.clone()
        )
        .map_err(Error::Config)?,
    ));
    let router = Arc::new(RwLock::new(Router::new(
        &mut config.router,
        dns_client.clone(),
    )));
    let dispatcher = Arc::new(Dispatcher::new(
        outbound_manager.clone(),
        router.clone(),
        dns_client.clone(),
    ));
    let nat_manager = Arc::new(NatManager::new(dispatcher.clone()));
    let inbound_manager =
        InboundManager::new(&config, dispatcher, nat_manager).map_err(Error::Config)?;
    let mut inbound_net_runners = inbound_manager
        .get_network_runners()
        .map_err(Error::Config)?;
    runners.append(&mut inbound_net_runners);

    let network_changed: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    let net_info = if inbound_manager.has_tun_listener() && inbound_manager.tun_auto() {
        sys::get_net_info()
    } else {
        sys::NetInfo::default()
    };
    //#[cfg(all(any(target_os = "windows")))]
    // let net_info = if inbound_manager.has_tun_listener() && inbound_manager.tun_auto() {
    // let net_info =   sys::get_net_info();
    //let net_info = sys_raw::NetInfo::default();
    // } else {
    // sys::NetInfo::default()
    // };
    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    {
        if let sys::NetInfo {
            default_interface: Some(iface),
            ..
        } = &net_info
        {
            let binds = if let Ok(v) = std::env::var("OUTBOUND_INTERFACE") {
                format!("{},{}", v, iface)
            } else {
                iface.clone()
            };
            println!("OUTBOUND_INTERFACE: {:?}", binds);
            std::env::set_var("OUTBOUND_INTERFACE", binds);
        }
    }
    // #[cfg(all(feature = "inbound-tun", target_os = "windows"))]
    // {
    //     let interface = common::cmd::get_default_interface().unwrap();
    //     std::env::set_var("OUTBOUND_INTERFACE", interface);

    // }

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
    if let Ok(r) = inbound_manager.get_tun_runner() {
        runners.push(r);
    }

    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    sys::post_tun_creation_setup(&net_info);
    // #[cfg(target_os = "windows")]{
    // sys::post_tun_creation_setup(&net_info);
    // }
  /*  #[cfg(all(
        feature = "inbound-tun",
        any(target_os = "macos", target_os = "linux",)
    ))]*/
    #[cfg(all(
    feature = "inbound-tun",
    any(target_os = "android")
    ))]
    {
        use futures::stream::StreamExt;
        use signal_hook::consts::signal::*;
        use signal_hook_tokio::Signals;
        // use std::thread;
        // use signal_hook::iterator::Signals;

        /*        async fn handle_signals(
            mut signals: Signals,
            _old_net_info: &NetInfo,
            new_net_info: &NetInfo,
            shutdown_tx: mpsc::Sender<()>,
        ) {
            while let Some(signal) = signals.next().await {
                match signal {
                    SIGALRM => {
                        log::trace!("signal received {}", &SIGALRM);
                        // sys::post_tun_completion_setup(old_net_info);
                        // thread::sleep(std::time::Duration::from_secs(1));
                        // sys::post_tun_reload_setup(new_net_info);

                    }
                    SIGTERM
                    // | SIGINT | SIGQUIT
                    => {
                        log::trace!("signal received {}", &SIGTERM);
                        // sys::post_tun_completion_setup(new_net_info);
                        if let Err(e) = shutdown_tx.send(()).await {
                            log::warn!("sending shutdown signal failed: {}", e);
                        }
                        return;
                    }
                    _ => unreachable!(),
                }
            }
        }*/

        let mut signals = Signals::new(&[SIGTERM, SIGPIPE, SIGALRM])?;
        let signals_handle = signals.handle();
        // let net_info = net_info.clone();
        let shutdown_tx = shutdown_tx.clone();
        let network_changed = network_changed.clone();

        // let new_net_info = if inbound_manager.has_tun_listener() && inbound_manager.tun_auto() {
        //     sys::get_net_info()
        // } else {
        //     sys::NetInfo::default()
        // };

        tokio::spawn(async move {
            use bytes::BytesMut;
            use protobuf::Message;
            use protocol::{
                generated::notification::{StatusNotification, StatusRequest},
                unpack_msg_frame,
            };
            use std::{net::SocketAddr, str::FromStr, time::Duration};
            use tokio::{io::AsyncReadExt, time::timeout};
            use udp_stream::UdpListener;
            const UDP_BUFFER_SIZE: usize = 1024; // 17kb
            const UDP_TIMEOUT: u64 = 10 * 1000; // 10sec
            let listener = UdpListener::bind(SocketAddr::from_str("127.0.0.1:11771").unwrap())
                .await
                .expect("12345");
            // let mut buf = BytesMut::with_capacity(UDP_BUFFER_SIZE);
            let mut buf = vec![0u8; UDP_BUFFER_SIZE];
            let mut interval = interval_at(
                Instant::now() + Duration::from_secs(7),
                Duration::from_secs(5),
            );
            let mut should_exit = true;
            loop {
                tokio::select! {
                     Some(signal) = signals.next() =>{
                        match signal {
                            // SIGPIPE => {
                            //     log::trace!("signal received {}", &SIGPIPE);
                            //     // sys::post_tun_completion_setup(old_net_info);
                            //     // thread::sleep(std::time::Duration::from_secs(1));
                            //     // sys::post_tun_reload_setup(new_net_info);
                            //     network_changed.store(true, Ordering::Relaxed);
                            //     if let Err(e) = shutdown_tx.send(()).await {
                            //         log::warn!("sending shutdown signal failed: {}", e);
                            //     }
                            //     return;
                            // }
                            SIGALRM =>{
                                log::trace!("signal received {}", &SIGALRM);
                                // sys::post_tun_completion_setup(new_net_info);
                                network_changed.store(true, Ordering::Relaxed);
                                if let Err(e) = shutdown_tx.send(()).await {
                                    log::warn!("sending shutdown signal failed: {}", e);
                                }
                                break;
                            }
                            SIGTERM
                            // | SIGINT | SIGQUIT
                            => {
                                log::trace!("signal received {}", &SIGTERM);
                                // println!("signal received {}", &SIGTERM);
                                // sys::post_tun_completion_setup(new_net_info);
                                if let Err(e) = shutdown_tx.send(()).await {
                                    log::warn!("sending shutdown signal failed: {}", e);
                                }
                                break;
                            }
                            _ => unreachable!(),
                        }
                    }
                    Ok((mut stream, _)) = listener.accept() =>{
                        // buf.clear();
                        match timeout(Duration::from_millis(UDP_TIMEOUT), stream.read(&mut buf))
                    .await
                    .unwrap()
                        {
                            Ok(len) => {
                                let mut buf = BytesMut::from(&buf[..len]);
                                unpack_msg_frame(&mut buf).unwrap();
                                println!("udp received: {:?}", buf);
                              let in_msg = StatusRequest::parse_from_bytes(&buf).unwrap();
                                println!("protocol: {:?}", in_msg.status);
                            if in_msg.status.value()  == StatusNotification::Running.value(){
                                should_exit = false;
                            }
                        },
                            Err(_) => {
                                stream.shutdown();
                                continue;
                            }
                        };
                    }
                    _ = interval.tick() => {
                        if should_exit{
                            if let Err(e) = shutdown_tx.send(()).await {
                              log::warn!("sending shutdown signal failed: {}", e);
                            }
                            break
                        }
                        should_exit = true;
                    }
                }
            }
            signals_handle.close();
        });
    }

    let runtime_manager = RuntimeManager::new(
        /*        #[cfg(feature = "auto-reload")]
        rt_id,*/
        // config_path,
        // reload_tx,
        shutdown_tx,
        // router,
        // dns_client,
        // outbound_manager,
    );

    /*    // Monitor config file changes.
    #[cfg(feature = "auto-reload")]
    {
        if let Err(e) = runtime_manager.new_watcher() {
            log::warn!("start config file watcher failed: {}", e);
        }
    }*/

    drop(config); // explicitly free the memory

    // The main task joining all runners.
    tasks.push(Box::pin(async move {
        futures::future::join_all(runners).await;
    }));

    // Monitor shutdown signal.
    tasks.push(Box::pin(async move {
        let _ = shutdown_rx.recv().await;
    }));

    // Monitor ctrl-c exit signal.
    #[cfg(feature = "ctrlc")]
    tasks.push(Box::pin(async move {
        let _ = tokio::signal::ctrl_c().await;
    }));

    RUNTIME_MANAGER
        .lock()
        .map_err(|_| Error::RuntimeManager)?
        .insert(INSTANCE_ID, runtime_manager);

    log::trace!("added runtime {}", &INSTANCE_ID);

    rt.block_on(futures::future::select_all(tasks));

    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    {
        if !network_changed.load(Ordering::Relaxed) {
            log::trace!("runtime {} quit as untouched os route", &INSTANCE_ID);
            sys::post_tun_completion_setup(&net_info);
        }
    }

    // #[cfg(all(any(target_os = "windows")))]
    // sys::post_tun_completion_setup(&net_info);

    rt.shutdown_background();

    RUNTIME_MANAGER
        .lock()
        .map_err(|_| Error::RuntimeManager)?
        .remove(&INSTANCE_ID);

    log::trace!("removed runtime {}", &INSTANCE_ID);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_restart() {
        let conf = r#"
[General]
loglevel = trace
dns-server = 1.1.1.1
socks-interface = 127.0.0.1
socks-port = 1080
# tun = auto

[Proxy]
Direct = direct
"#;

        for i in 1..10 {
            thread::spawn(move || {
                let opts = StartOptions {
                    config: Config::Str(conf.to_string()),
                };
                start(opts);
            });
            thread::sleep(std::time::Duration::from_secs(5));
            shutdown(0);
            loop {
                thread::sleep(std::time::Duration::from_secs(2));
                if !is_running(0) {
                    break;
                }
            }
        }
    }
}
