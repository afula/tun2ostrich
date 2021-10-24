use std::process::exit;

use argh::FromArgs;

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

    if let Err(e) = ostrich::util::run_with_options(args.config) {
        println!("start ostrich failed: {}", e);
        exit(1);
    }
}
