use crate::config;

use anyhow::{anyhow, Result};
#[cfg(any(target_os = "ios"))]
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
#[cfg(any(target_os = "ios", target_os = "linux", target_os = "macos"))]
pub fn setup_logger(config: &config::Log) -> Result<()> {
    let loglevel = match config.level {
        config::Log_Level::TRACE => log::LevelFilter::Trace,
        config::Log_Level::DEBUG => log::LevelFilter::Debug,
        config::Log_Level::INFO => log::LevelFilter::Info,
        config::Log_Level::WARN => log::LevelFilter::Warn,
        config::Log_Level::ERROR => log::LevelFilter::Error,
    };
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
        // .filter(None, loglevel)
        .filter(Some("ostrich"), loglevel)
        .init();
    Ok(())
}
#[cfg(target_os = "android")]
pub fn setup_logger(config: &config::Log) -> Result<()> {
    use android_logger::{Config, FilterBuilder};
    use log::Level;

    let loglevel = match config.level {
        config::Log_Level::TRACE => Level::Trace,
        config::Log_Level::DEBUG => Level::Debug,
        config::Log_Level::INFO => Level::Info,
        config::Log_Level::WARN => Level::Warn,
        config::Log_Level::ERROR => Level::Error,
    };
    android_logger::init_once(
        Config::default()
            .with_min_level(loglevel) // limit log level
            .with_tag("ostrich")
            .format(|f, record| {
                write!(
                    f,
                    "ostrich:{}:{} {} [{}] - {}",
                    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.level(),
                    record.args()
                )
            }).with_filter(FilterBuilder::new().filter(Some("ostrich"), loglevel.to_level_filter()).build()), // logs will show under mytag tag
                /*            .with_filter(
                    // configure messages for specific crate
                    FilterBuilder::new()
                        .parse("debug,hello::crate=error")
                        .build(),
                )*/
    );
    Ok(())
}
