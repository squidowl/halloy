use std::env;

use data::log;

pub fn setup(is_debug: bool) -> Result<(), log::Error> {
    let level_filter = env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::parse::<log::Level>)
        .transpose()?
        .unwrap_or(log::Level::Debug)
        .to_level_filter();

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
        .level_for("data", level_filter)
        .level_for("halloy", level_filter);

    if is_debug {
        logger = logger.chain(std::io::stdout());
    } else {
        let log_file = log::file()?;

        logger = logger.chain(log_file);
    }

    logger.apply()?;
    Ok(())
}
