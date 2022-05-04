#![feature(once_cell)]

use anyhow::anyhow;
use app::{
    dispatcher::Dispatcher, dns_client::DnsClient, inbound::manager::InboundManager,
    nat_manager::NatManager, outbound::manager::OutboundManager, router::Router,
};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Once;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

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

// #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows",target_os = "ios"))]
#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

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
    shutdown_tx: mpsc::Sender<()>,
}

impl RuntimeManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(shutdown_tx: mpsc::Sender<()>) -> Arc<Self> {
        Arc::new(Self { shutdown_tx })
    }

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

pub fn start(
    opts: StartOptions,
    #[cfg(target_os = "windows")] wintun_path: String,
    #[cfg(target_os = "windows")] tun2socks_path: String,
) -> Result<(), Error> {
    println!("start with options:\n{:#?}", opts);

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

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
    let mut ipset = Vec::from(config.dns.servers.clone());
    for (_, ips) in &config.dns.hosts {
        ipset.append(&mut ips.values.to_owned())
    }

    let inbound_manager = InboundManager::new(
        #[cfg(target_os = "windows")]
        ipset.clone(),
        &config,
        dispatcher,
        nat_manager,
        #[cfg(target_os = "windows")]
        wintun_path,
        #[cfg(target_os = "windows")]
        tun2socks_path,
    )
    .map_err(Error::Config)?;

    let mut inbound_net_runners = inbound_manager
        .get_network_runners()
        .map_err(Error::Config)?;
    runners.append(&mut inbound_net_runners);


    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    let net_info = if inbound_manager.has_tun_listener() && inbound_manager.tun_auto() {
        sys::get_net_info()?
    } else {
        sys::NetInfo::default()
    };
    let mut net_info = Arc::new(Mutex::new(net_info));
    //#[cfg(all(any(target_os = "windows")))]
    // let net_info = if inbound_manager.has_tun_listener() && inbound_manager.tun_auto() {
    // let net_info =   sys::get_net_info();
    //let net_info = sys_raw::NetInfo::default();
    // } else {
    // sys::NetInfo::default()
    // };
    /*    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
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
    }*/
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

    /*    #[cfg(all(feature = "inbound-tun", any(target_os = "macos", target_os = "linux")))]
    sys::post_tun_creation_setup(&net_info);*/
    // #[cfg(target_os = "windows")]{
    // sys::post_tun_creation_setup(&net_info);
    // }
    #[cfg(all(
        feature = "inbound-tun",
        any(target_os = "macos", target_os = "linux",)
    ))]
    {
        use futures::stream::StreamExt;
        use signal_hook::consts::signal::*;
        use signal_hook_tokio::Signals;
        let mut signals = Signals::new(&[SIGTERM, SIGPIPE, SIGALRM])?;
        let signals_handle = signals.handle();
        let shutdown_tx = shutdown_tx.clone();
        let mut net_info = net_info.clone();

        tokio::spawn(async move {
            use if_watch::{IfEvent, IfWatcher};
            let mut if_set = IfWatcher::new().await.unwrap();

            let if_fut = Box::pin(async {
                while let Ok(event) = Pin::new(&mut if_set).await {
                    // #[cfg(target_os = "macos")]{
                    match event {
                        IfEvent::Up(up_ip) => {
                            if up_ip.addr().is_ipv4()
                                && up_ip.addr().to_string() != "172.7.0.2".to_string()
                                && up_ip.addr().to_string() != "172.7.0.1".to_string()
                                && up_ip.addr().to_string() != "127.0.0.1".to_string()
                            {
                                'net: loop {
                                    match sys::get_net_info() {
                                        Ok(sys_net) => {
                                            if let sys::NetInfo {
                                                default_interface: Some(iface),
                                                default_ipv4_address: Some(ip),
                                                ..
                                            } = &sys_net
                                            {
                                                #[cfg(target_os = "macos")]
                                                {
                                                    if ip != "172.7.0.2" {
                                                        println!("UP: after network interface changed,the new ipv4 is: {}", ip);
                                                        std::env::set_var(
                                                            "OUTBOUND_INTERFACE",
                                                            iface,
                                                        );
                                                        println!(
                                                            "OUTBOUND_INTERFACE: {:?}",
                                                            std::env::var("OUTBOUND_INTERFACE")
                                                        );
                                                        sys::post_tun_creation_setup(&sys_net);
                                                        *net_info.lock().unwrap() = sys_net;
                                                        break 'net;
                                                    }
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_millis(100),
                                                    )
                                                    .await;
                                                    continue 'net;
                                                }
                                                #[cfg(target_os = "linux")]
                                                {
                                                    if ip != "172.7.0.2" {
                                                        println!("UP: after network interface changed,the new ipv4 is: {}", ip);
                                                        std::env::set_var(
                                                            "OUTBOUND_INTERFACE",
                                                            iface,
                                                        );
                                                        println!(
                                                            "OUTBOUND_INTERFACE: {:?}",
                                                            std::env::var("OUTBOUND_INTERFACE")
                                                        );
                                                        sys::post_tun_creation_setup(&sys_net);
                                                        *net_info.lock().unwrap() = sys_net;
                                                        break 'net;
                                                    }
                                                    break 'net;
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            tokio::time::sleep(std::time::Duration::from_millis(
                                                7000,
                                            ))
                                            .await;
                                            continue 'net;
                                        }
                                    }
                                }
                            }
                        }
                        IfEvent::Down(dw_ip) => {
                            /*                                println!("down: ip({:?}, default_ip({:?}),default_init_ip: {})", &dw_ip, &default_ipv4,&default_init_ipv4);
                                                            if default_ipv4 == dw_ip.addr().to_string() && default_ipv4 != default_init_ipv4{
                                                                network_changed.store(true, Ordering::Relaxed);
                                                                println!("DOWN: tag network changed flag");
                            /*                                    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                                                                match sys::get_net_info() {
                                                                    Ok(net_info) => {
                                                                        if let sys::NetInfo {
                                                                            default_interface: Some(iface),
                                                                            default_ipv4_address: Some(ip),
                                                                            ..
                                                                        } = &net_info
                                                                        {
                                                                            if ip != "172.7.0.2"{
                                                                                default_ipv4 = ip.to_owned();
                                                                                println!("DOWN: after network interface changed,the new default ipv4 is: {}", default_ipv4);
                                                                                std::env::set_var("OUTBOUND_INTERFACE", iface);
                                                                                println!(
                                                                                    "OUTBOUND_INTERFACE: {:?}",
                                                                                    std::env::var("OUTBOUND_INTERFACE")
                                                                                );
                                                                                sys::post_tun_creation_setup(&net_info);
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(_) => {}
                                                                }*/
                                                            }*/
                        }
                    }
                    // } //macos
                }
            });
            let signal_fut = Box::pin(async {
                while let Some(signal) = signals.next().await {
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
/*                        SIGALRM =>{
                            log::trace!("signal received {}", &SIGALRM);
                            // sys::post_tun_completion_setup(new_net_info);
                            network_changed.store(true, Ordering::Relaxed);
                            if let Err(e) = shutdown_tx.send(()).await {
                                log::warn!("sending shutdown signal failed: {}", e);
                            }
                            break;
                        }*/
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
                signals_handle.close();
            });
            futures::future::select(if_fut, signal_fut).await;
            Ok(()) as std::io::Result<()>
        });
    }
    #[cfg(all(feature = "inbound-tun", any(target_os = "windows",)))]
    {
        tokio::spawn(async move {
            use crate::common::cmd;
            use if_watch::{IfEvent, IfWatcher};
            use std::pin::Pin;
            use std::process::Command;

            /*
                        // println!("gateway: {:?}", gateway);
                        let mut if_index: u32 = 0;
                        let mut if_name = String::default();
                        let mut default_ipv4 = String::default();

                        let mut adapters = ipconfig::get_adapters().unwrap();
                        adapters.sort_by(|ip1, ip2| ip1.ipv4_metric().cmp(&ip2.ipv4_metric()));
                        for adapter in adapters {
            /*                 println!(
                                "{}: IfType: {:?}  IPs: {:?} - IPv4 metric: {} IPv6 metric: {} IPV6 index: {:?}, Dns server: {:?}, Gateways: {:?}",
                                adapter.friendly_name(),
                                adapter.if_type(),
                                adapter.ip_addresses(),
                                adapter.ipv4_metric(),
                                adapter.ipv6_metric(),
                                adapter.ipv6_if_index(),
                                adapter.dns_servers(),
                                adapter.gateways()
                            ); */
                            if adapter.gateways().contains(&gateway.parse().unwrap()) {
                                if_index = adapter.ipv6_if_index();
                                if_name = adapter.adapter_name().to_string();
                                for ip in adapter.ip_addresses(){
                                    if ip.is_ipv4(){
                                        default_ipv4 = ip
                                    }
                                }
                            }
                        }

                        let prefix = 32;
                        use crate::proxy::tun::win::route::route_add_with_if;
                        println!("if_index: {:?}, if_name: {:?}, default ip: {:?} ",if_index,if_name,&default_ipv4); */
            tokio::time::sleep(std::time::Duration::from_secs(7)).await;
            let mut if_set = IfWatcher::new().await.unwrap();
            loop {
                let if_event = Pin::new(&mut if_set).await?;
                println!("network ifterface event: {:?}", if_event);
                match if_event {
                    IfEvent::Up(ip) => {
                        if ip.addr().is_ipv4()
                            && ip.addr().to_string() != "172.7.0.2".to_string()
                            && ip.addr().to_string() != "172.7.0.1".to_string()
                            && ip.addr().to_string() != "127.0.0.1".to_string()
                        // && ip.addr().to_string() != init_gateway
                        {
                            /*                             for v in &ipset {
                                let ip: std::net::Ipv4Addr =
                                    v.parse().expect(&format!("wrong ip address:{}", &ip));
                                let ip_u32: u32 = ip.into();
                                route_add_with_if(ip_u32, ip_mask, 0, if_index)
                                    .map_err(|e| {
                                        println!("{:?}", e);
                                        e
                                    })
                                    .expect(
                                        &format!("could not add ip: {:?} to route table", &ip)
                                            .as_str(),
                                    );
                                let out = Command::new("route")
                                    .arg("delete")
                                    .arg(v)
                                    .status()
                                    .expect("failed to execute command");
                                println!("process finished with: {}", out);
                            } */
                            let out = Command::new("route")
                                .arg("delete")
                                .arg("0.0.0.0")
                                .arg("172.7.0.1")
                                .status()
                                .expect("failed to execute command");
                            println!("route delete command finished with: {}", out);
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            match cmd::get_default_ipv4_gateway() {
                                Ok(gw) => {
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
                                    println!("setup tun device command finished with: {}", out);
                                    for v in &ipset {
                                        let out = Command::new("route")
                                            .arg("add")
                                            .arg(v)
                                            .arg(&gw)
                                            .arg("metric")
                                            .arg("5")
                                            .status()
                                            .expect("failed to execute command");
                                        println!("route add command finished with: {}", out);
                                    }
                                }
                                Err(e) => {
                                    println!("network changed,cant get gateway:{:?}", e);
                                    continue;
                                }
                            }
                        }
                    }
                    IfEvent::Down(ip) => {}
                }
            }
            Ok(()) as std::io::Result<()>
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
        // if !network_changed.load(Ordering::Relaxed) {
        //     log::trace!("runtime {} quit as untouched os route", &INSTANCE_ID);
        //     sys::post_tun_completion_setup(&net_info);
        // }
        let net_info = net_info.lock().unwrap();
        let net = sys::NetInfo{
            default_ipv4_gateway: net_info.default_ipv4_gateway.clone(),
            default_ipv6_gateway: net_info.default_ipv6_gateway.clone(),
            default_ipv4_address: net_info.default_ipv4_address.clone(),
            default_ipv6_address: net_info.default_ipv6_address.clone(),
            ipv4_forwarding: net_info.ipv4_forwarding,
            ipv6_forwarding: net_info.ipv6_forwarding,
            default_interface: net_info.default_interface.clone()
        };

        if let sys::NetInfo {
            default_ipv4_address: Some(ip),
            ..
        } = &net
        {
            if ip != "172.7.0.2" {
                sys::post_tun_completion_setup(&net);
            }
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

/*            loop {
                tokio::select! {

/*                        Ok(event) = &mut if_set =>{
                           #[cfg(target_os = "macos")]{
                            println!("got if event: {:?}, default_ipv4: {:?}", event,&default_ipv4);
                            match event {
                                IfEvent::Up(up_ip) => {

                                      if up_ip.addr().is_ipv4()
                                        && up_ip.addr().to_string() != "172.7.0.2".to_string()
                                        && up_ip.addr().to_string() != "172.7.0.1".to_string()
                                        && up_ip.addr().to_string() != "127.0.0.1".to_string(){
                                            tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
                                            match sys::get_net_info(){
                                                Ok(net_info) =>{
                                                        if let sys::NetInfo {
                                                            default_interface: Some(iface),
                                                            default_ipv4_address: Some(ip),
                                                            ..
                                                        } = &net_info
                                                        {
                                                            // let binds = if let Ok(v) = std::env::var("OUTBOUND_INTERFACE") {
                                                            //     format!("{},{}", v, iface)
                                                            // } else {
                                                            //     iface.clone()
                                                            // };
                                                            default_ipv4 = ip.to_owned();
                                                            println!("UP after network interface changed,the new default ipv4 is: {},up_ip: {}", default_ipv4,&up_ip.addr().to_string()  );

                                                            std::env::set_var("OUTBOUND_INTERFACE", iface);
                                                            println!("OUTBOUND_INTERFACE: {:?}", std::env::var("OUTBOUND_INTERFACE"));
                                                        }
                                                        sys::post_tun_creation_setup(&net_info);
                                                }
                                                Err(_) =>{

                                                 }
                                             }
                                         }
                                }
                                IfEvent::Down(dw_ip) => {
                                    println!("down: ip({:?}, default_ip({:?}))", &dw_ip,&default_ipv4);
                                    if default_ipv4 == dw_ip.addr().to_string(){
                                        network_changed.store(true, Ordering::Relaxed);
                                        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                                        match sys::get_net_info(){
                                            Ok(net_info) =>{
                                            // #[cfg(target_os = "macos")]{
                                                    if let sys::NetInfo {
                                                        default_interface: Some(iface),
                                                        default_ipv4_address: Some(ip),
                                                        ..
                                                    } = &net_info
                                                    {
                                                        // let binds = if let Ok(v) = std::env::var("OUTBOUND_INTERFACE") {
                                                        //     format!("{},{}", v, iface)
                                                        // } else {
                                                        //     iface.clone()
                                                        // };
                                                        default_ipv4 = ip.to_owned();
                                                        println!("after network interface changed,the new default ipv4 is: {}", default_ipv4);

                                                        std::env::set_var("OUTBOUND_INTERFACE", iface);
                                                        println!("OUTBOUND_INTERFACE: {:?}", std::env::var("OUTBOUND_INTERFACE"));
                                                    }
                                                    sys::post_tun_creation_setup(&net_info);
                                            // }
                                            }
                                            Err(_) =>{

                                             }
                                        }
                                    }

                                }
                            }
                        }
                    }*/
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
                /*                    Ok((mut stream, _)) = listener.accept() =>{
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
                                    }*/
                                }
            }*/
