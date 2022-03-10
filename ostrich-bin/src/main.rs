use std::process::exit;

use argh::FromArgs;
use ostrich::common::cmd;
use std::process::Command;
use std::thread;

#[derive(FromArgs)]
/// A lightweight and fast proxy utility
struct Args {
    /// the configuration file
    #[argh(option, short = 'c', default = "String::from(\"config.conf\")")]
    config: String,
    /*   /// enables auto reloading when config file changes
    #[cfg(feature = "auto-reload")]
    #[argh(switch)]
    auto_reload: bool,

    /// runs in a single thread
    #[argh(switch)]
    single_thread: bool,

    /// sets the stack size of runtime worker threads
    #[argh(option, default = "default_thread_stack_size()")]
    thread_stack_size: usize,

    /// tests the configuration and exit
    #[argh(switch, short = 'T')]
    test: bool,

    /// tests the connectivity of the specified outbound
    #[argh(option, short = 't')]
    test_outbound: Option<String>,

    /// prints version
    #[argh(switch, short = 'V')]
    version: bool,*/
}

fn main() {
    let args: Args = argh::from_env();
    // tun2socks -device tun://TunMax -proxy {}
    // cmd::get_default_ipv4_gateway();

    // let handler1 = thread::spawn(|| {
    //     let p = Command::new("misc/tun2socks")
    //         .arg("-device")
    //         .arg("tun://utun233")
    //         .arg("-proxy")
    //         .arg("socks5://127.0.0.1:1086")
    //         // flag.StringVar(&key.LogLevel, "loglevel", "info", "Log level [debug|info|warning|error|silent]")
    //         .arg("-loglevel")
    //         .arg("warning")
    //         .status()
    //         .expect("failed to execute process");
    //     println!("process finished with: {}", p);
    // });

    // thread::sleep(std::time::Duration::from_millis(3000));

    let handler0 = thread::spawn(|| {
        if let Err(e) = ostrich::util::run_with_options(args.config) {
            println!("start ostrich failed: {}", e);
            exit(1);
        }
    });

    // netsh interface ip set address TunMax static 240.255.0.2 255.255.255.0 11.0.68.1 3
    #[cfg(target_os = "windows")]
    {
        let gateway = ostrich::common::cmd::get_default_ipv4_gateway().unwrap();
        println!("gateway: {:?}", gateway);

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
        println!("process finished with: {}", out);
        let out = Command::new("route")
            .arg("add")
            .arg("1.1.1.1")
            .arg(gateway.clone())
            .arg("metric")
            .arg("5")
            .status()
            .expect("failed to execute command");
        println!("process finished with: {}", out);

        let out = Command::new("route")
            .arg("add")
            .arg("45.77.197.43")
            .arg(gateway)
            .arg("metric")
            .arg("5")
            .status()
            .expect("failed to execute command");
        println!("process finished with: {}", out);
    }

    handler0.join().unwrap();
    // handler1.join().unwrap();
}
