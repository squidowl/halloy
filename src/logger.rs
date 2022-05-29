use thiserror::Error;

use data::config::{self, Config};

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error")]
    ConfigError(config::Error),
    #[error("io error")]
    IoError(std::io::Error),
    #[error("logger error")]
    LogError(log::SetLoggerError),
}

pub fn setup(is_debug: bool) -> Result<(), Error> {
    let mut logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}:{} -- {}",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("data", log::LevelFilter::Trace)
        .level_for("halloy", log::LevelFilter::Trace);

    if is_debug {
        logger = logger.chain(std::io::stdout());
    } else {
        use std::fs::OpenOptions;

        let config_dir = Config::config_dir().map_err(|e| Error::ConfigError(e))?;

        let log_file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .truncate(true)
            .open(config_dir.join("halloy.log"))
            .map_err(|e| Error::IoError(e))?;

        logger = logger.chain(log_file);
    }

    logger.apply().map_err(|e| Error::LogError(e))?;
    Ok(())
}
