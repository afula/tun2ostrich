use crate::config;

use anyhow::{anyhow, Result};

pub fn setup_logger(config: &config::Log) -> Result<()> {
    let loglevel = match config.level {
        config::Log_Level::TRACE => log::LevelFilter::Trace,
        config::Log_Level::DEBUG => log::LevelFilter::Debug,
        config::Log_Level::INFO => log::LevelFilter::Info,
        config::Log_Level::WARN => log::LevelFilter::Warn,
        config::Log_Level::ERROR => log::LevelFilter::Error,
    };

    let mut dispatch = fern::Dispatch::new()
        .format(move |out, message, record| {
            if *crate::option::LOG_NO_COLOR {
                out.finish(format_args!(
                    "[{date}][{level}] {message}",
                    date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    level = record.level(),
                    message = message,
                ))
            } else {
                use fern::colors::{Color, ColoredLevelConfig};
                let colors_line = ColoredLevelConfig::new()
                    .error(Color::Red)
                    .warn(Color::Yellow)
                    .info(Color::White)
                    .debug(Color::White)
                    .trace(Color::BrightBlack);

                let colors_level = colors_line.info(Color::Green);
                out.finish(format_args!(
                    // "{color_line}[{date}][{level}{color_line}][{target}] {message}\x1B[0m",
                    "{color_line}[{date}][{level}{color_line}] {message}\x1B[0m",
                    color_line = format_args!(
                        "\x1B[{}m",
                        colors_line.get_color(&record.level()).to_fg_str()
                    ),
                    date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    // target = record.target(),
                    level = colors_level.color(record.level()),
                    message = message,
                ))
            }
        })
        .level(log::LevelFilter::Warn)
        .level_for("ostrich", loglevel);

    match config.output {
        config::Log_Output::CONSOLE => {
            #[cfg(any(target_os = "ios", target_os = "android"))]
            {
                let console_output = fern::Output::writer(
                    Box::new(crate::mobile::logger::ConsoleWriter::default()),
                    "\n",
                );
                dispatch = dispatch.chain(console_output);
            }
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            {
                dispatch = dispatch.chain(fern::Output::stdout("\n"));
            }
            #[cfg(target_os = "macos")]
            if *crate::option::LOG_CONSOLE_OUT {
                let console_output = fern::Output::writer(
                    Box::new(crate::mobile::logger::ConsoleWriter::default()),
                    "\n",
                );
                dispatch = dispatch.chain(console_output);
            }
        }
        config::Log_Output::FILE => {
            let f = fern::log_file(&config.output_file)?;
            let file_output = fern::Output::file(f, "\n");
            dispatch = dispatch.chain(file_output);
        }
    }

    if let Err(e) = dispatch.apply() {
        return Err(anyhow!("apply logger config failed: {}", e));
    }

    Ok(())
}

/*

use log::LevelFilter;
use log4rs::{
    append::rolling_file::policy::compound::{
        roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
    },
    filter::threshold::ThresholdFilter,
};

// fn init_logger() -> anyhow::Result<()> {
//     use log::LevelFilter;
//     use log4rs::append::file::FileAppender;
//     use log4rs::config::{Appender, Config, Root};
//     use log4rs::encode::pattern::PatternEncoder;
//
//     let logfile = FileAppender::builder()
//                                                             // {l} {m} at {M} in {f}:{L}
//         .encoder(Box::new(PatternEncoder::new("[{d} {l} {M}] {m} in {f}:{L}\n")))
//         .build("gurk.log")?;
//
//     let config = Config::builder()
//         .appender(Appender::builder().build("logfile", Box::new(logfile)))
//         .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;
//
//     log4rs::init_config(config)?;
//     Ok(())
// }

pub fn setup_logger(config: &config::Log) -> Result<()> {
    use std::io::Write;
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        // .filter(Some("logger_example"), LevelFilter::Debug)
        .init();

    // let loglevel = match config.level {
    //     config::Log_Level::TRACE => log::LevelFilter::Trace,
    //     config::Log_Level::DEBUG => log::LevelFilter::Debug,
    //     config::Log_Level::INFO => log::LevelFilter::Info,
    //     config::Log_Level::WARN => log::LevelFilter::Warn,
    //     config::Log_Level::ERROR => log::LevelFilter::Error,
    // };

    // use log4rs::{
    //     append::rolling_file::RollingFileAppender,
    //     config::{Appender, Config, Root},
    //     encode::pattern::PatternEncoder
    // };
    // const K: u64 = 1024;
    // const M: u64 = K * K;
    // const FILE_SIZE: u64 = 5 * M; // as max log file size to roll
    // const FILE_COUNT: u32 = 7;

    // // let window_size = 3; // log0, log1, log2
    // let fixed_window_roller = FixedWindowRoller::builder().build("log{}", FILE_COUNT).unwrap();
    // let size_trigger = SizeTrigger::new(FILE_SIZE);
    // let compound_policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));

    // let config = Config::builder()
    //     .appender(
    //         Appender::builder().filter(Box::new(ThresholdFilter::new(loglevel)))).build(
    //             "logfile",
    //             Box::new(
    //                 RollingFileAppender::builder()
    //                     .encoder(Box::new(PatternEncoder::new("[{d} {l} {M}] {m} in {f}:{L}\n")))
    //                     .build(&log_path, Box::new(compound_policy))
    //                     .unwrap()
    //             )
    //         )
    //     )
    //     .build(Root::builder().appender("logfile").build(from_usize(level as usize)))
    //     .unwrap();
    // log4rs::init_config(config).map_err(|e| Error::Eor(anyhow::anyhow!("{:?}", e)))?;
    log_panics::init();

    Ok(())
}
 */
