use argh::FromArgs;
use ostrich::common::cmd;
use std::process::exit;
use std::process::Command;
use std::time::Duration;
use tokio::runtime;
use tokio::time::sleep;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the runtime
    let rt = runtime::Builder::new_current_thread().enable_time().build()?;
    // use_max_file_limit();
    println!("#0");
    rt.block_on(async {
        let handle = std::thread::spawn(||{
            println!("#1");
            let p = Command::new("./ostrich_worker")
                .arg("-c")
                .arg("tun_auto_win.json")
                .status()
                .expect("cant start ostrich_manager");
            if !p.success() {
                println!("init worker failed")
            }
        }
        );
        loop {
            sleep(Duration::from_secs(1)).await;
            println!("timer is running");
            let interface =
                cmd::get_default_interface().expect("cant get default network interface");
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
                let p = Command::new("./ostrich_worker")
                    .arg("-c")
                    .arg("tun_auto_win.json")
                    .status()
                    .expect("cant start ostrich_manager");
                if !p.success() {
                    println!("init worker failed")
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
        Err(e) => println!("Error while increasing file limit: {}", e)
    }
}
