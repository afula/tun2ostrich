#[cfg(not(target_os = "windows"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
use bytes::BytesMut;
use futures::stream::StreamExt;
use ostrich::common::cmd;
use protocol::{build_running_notification_req_cmd, pack_msg_frame};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tokio::runtime;
use tokio::time::sleep;
use tokio::{net::UdpSocket, time::timeout};

pub const DEFAULT_COMMAND_ADDR: &str = "127.0.0.1:11771";
    // Create the runtime
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()?;
    // use_max_file_limit();
    rt.block_on(async {
        // let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        std::thread::spawn(|| {
            let p = Command::new("sudo")
                .arg("./ostrich_worker")
                .arg("-c")
                .arg("latest.json")
                .stderr(Stdio::null())
                .stdout(Stdio::null())
                .stdin(Stdio::null())
                .status()
                .expect("cant start ostrich_manager");
            if !p.success() {
                println!("init worker failed")
            }
        });

        tokio::spawn(async move {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let mut data = BytesMut::default();
            build_running_notification_req_cmd(&mut data).await.unwrap();
            println!("#0 data len: {}", data.len());
            let msg = pack_msg_frame(data.as_ref());
            println!("#1 data len: {}", msg.len());

            loop {
                sleep(Duration::from_secs(3)).await;
                if let Ok(interface) = cmd::get_default_interface_v2() {
                    println!("default network interface: {:?}", &interface);
                    if interface != "utun233" {
                        println!("network changed");
                        let p = Command::new("killall")
                            .arg("-13")
                            .arg("ostrich_worker")
                            .stderr(Stdio::null())
                            .stdout(Stdio::null())
                            .stdin(Stdio::null())
                            .status()
                            .expect("cant send network signal");
                        if !p.success() {
                            println!("send network signal failed")
                        }
                        println!("send network changed signal");
                        std::thread::spawn(|| {
                            let p = Command::new("./ostrich_worker")
                                .arg("-c")
                                .arg("latest.json")
                                .stderr(Stdio::null())
                                .stdout(Stdio::null())
                                .stdin(Stdio::null())
                                .status()
                                .expect("cant start ostrich_manager");
                            if !p.success() {
                                println!("init worker failed")
                            }
                        });
                        println!("reload");
                    }
                }

                if let Err(_) = timeout(
                    Duration::from_secs(60),
                    socket.send_to(msg.as_ref(), DEFAULT_COMMAND_ADDR),
                )
                .await
                {
                    println!("cant send notification request")
                };
            }
        });
        let mut signals = Signals::new(&[SIGTERM]).unwrap();
        let signals_handle = signals.handle();
        while let Some(signal) = signals.next().await {
            match signal {
                    SIGTERM
                    // | SIGINT | SIGQUIT
                    => {
                        println!("signal received {}", &SIGTERM);
                        // sys::post_tun_completion_setup(new_net_info);
                        let p = Command::new("killall")
                            .arg("ostrich_worker")
                            .stderr(Stdio::null())
                            .stdout(Stdio::null())
                            .stdin(Stdio::null())
                            .status()
                            .expect("cant send network signal");
                        if !p.success() {
                            println!("send network signal failed")
                        }

                        break;
                    }
                    _ => unreachable!(),
                }
        }
        signals_handle.close();
        println!("manager exits gracefully");
    });
    Ok(())
}
#[cfg(target_os = "windows")]
fn main(){}