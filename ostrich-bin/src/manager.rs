use argh::FromArgs;
use ostrich::common::cmd;
use std::process::exit;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::runtime;
use tokio::time::sleep;
use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the runtime
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    // use_max_file_limit();
    rt.block_on(async {
        let handle = std::thread::spawn(|| {
            let p = Command::new("./ostrich_worker")
                .arg("-c")
                .arg("latest.json")
                .status()
                .expect("cant start ostrich_manager");
            if !p.success() {
                println!("init worker failed")
            }
        });

        let mut signals = Signals::new(&[SIGTERM])?;
        let signals_handle = signals.handle();

        tokio::spawn(async move {
            // handle_signals(signals, &net_info, &new_net_info, shutdown_tx).await;
            while let Some(signal) = signals.next().await {
                match signal {
                    SIGTERM
                    // | SIGINT | SIGQUIT
                    => {
                        println!("signal received {}", &SIGTERM);
                        // sys::post_tun_completion_setup(new_net_info);
                        let p = Command::new("killall")
                            .arg("ostrich_worker")
                            .status()
                            .expect("cant send network signal");
                        if !p.success() {
                            println!("send network signal failed")
                        }
                        return;
                    }
                    _ => unreachable!(),
                }
            }
            signals_handle.close();
        });
        loop {
            sleep(Duration::from_secs(2)).await;
            if let Ok(interface) = cmd::get_default_interface_v2() {
                println!("default network interface: {:?}", &interface);
                if interface != "utun233" {
                    println!("network changed");
                    let p = Command::new("killall")
                        .arg("-13")
                        .arg("ostrich_worker")
                        .status()
                        .expect("cant send network signal");
                    if !p.success() {
                        println!("send network signal failed")
                    }
                    println!("send network changed signal");
                    let handle = std::thread::spawn(|| {
                        println!("#1");
                        let p = Command::new("./ostrich_worker")
                            .arg("-c")
                            .arg("latest.json")
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
        handle.join().unwrap();
    })
}

/// Set our current maximum-file limit to a large value, if we can.
///
/// Since we're going to be used as a proxy, we're likely to need a
/// _lot_ of simultaneous sockets.
///
/// # Limitations
///
/// Maybe this should take a value from the configuration instead.
///
/// This doesn't actually do anything on windows.
pub fn use_max_file_limit() {
    /// Default maximum value to set for our maximum-file limit.
    ///
    /// If the system supports more than this, we won't ask for it.
    /// This should be plenty for proxy usage, though relays and onion
    /// services (once supported) may need more.
    const DFLT_MAX_N_FILES: u64 = 16384;

    match rlimit::utils::increase_nofile_limit(DFLT_MAX_N_FILES) {
        Ok(n) => println!("Increased process file limit to {}", n),
        Err(e) => println!("Error while increasing file limit: {}", e),
    }
}
