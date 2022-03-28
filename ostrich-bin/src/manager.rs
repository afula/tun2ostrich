use futures::stream::StreamExt;
use ostrich::common::cmd;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tokio::runtime;
use tokio::time::sleep;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the runtime
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()?;
    // use_max_file_limit();
    rt.block_on(async {
        // let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
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

        tokio::spawn(async move {
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
