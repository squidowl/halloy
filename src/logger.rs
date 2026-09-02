use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{env, mem, thread};

use chrono::Utc;
use data::config;
pub use data::log::{Error, Record};
use log::Log;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub fn setup(
    config: config::Logs,
) -> Result<ReceiverStream<Vec<Record>>, Error> {
    let env_rust_log = env::var("RUST_LOG")
        .ok()
        .as_deref()
        .and_then(|rust_log| str::parse::<log::Level>(rust_log).ok());

    let file_sink = file_dispatch(config, env_rust_log);

    let (pane_sink, pane_receiver) = pane_dispatch(env_rust_log);

    let mut dispatch = fern::Dispatch::new().chain(pane_sink);

    if let Some(file_sink) = file_sink {
        dispatch = dispatch.chain(file_sink);
    }

    dispatch.apply()?;

    Ok(pane_receiver)
}

fn file_dispatch(
    config: config::Logs,
    env_rust_log: Option<log::Level>,
) -> Option<fern::Dispatch> {
    data::log::clear(config.max_file_count.saturating_sub(1));

    if config.max_file_count == 0 {
        return None;
    }

    data::log::file(config.file_timestamp).ok().map(|log_file| {
        let file_level_filter = env_rust_log
            .map_or(log::LevelFilter::from(config.file_level), |env| {
                env.to_level_filter()
            });

        fern::Dispatch::new()
            .format(move |out, message, record| {
                let timestamp_format = "%H:%M:%S%.3f";
                let timestamp = match config.file_timestamp {
                    config::logs::Timestamp::Local => {
                        chrono::Local::now().format(timestamp_format)
                    }
                    config::logs::Timestamp::Utc => {
                        chrono::Utc::now().format(timestamp_format)
                    }
                };

                if message
                    .as_str()
                    .is_none_or(|message| message.contains('\n'))
                {
                    let formatted_message = format!("{message}");

                    let message = if formatted_message.contains('\n') {
                        let mut lines = formatted_message.lines();

                        let mut message =
                            lines.next().unwrap_or_default().to_string();

                        for line in lines {
                            message = format!(
                                "{message}\n{}{line}",
                                if line.is_empty() {
                                    ""
                                } else {
                                    // Indent width of format!("{} {:5} -- ", timestamp, record.level())
                                    "                      "
                                }
                            );
                        }

                        message
                    } else {
                        formatted_message
                    };

                    out.finish(format_args!(
                        "{} {:>5} -- {}",
                        timestamp,
                        record.level(),
                        message
                    ));

                    return;
                }

                out.finish(format_args!(
                    "{} {:>5} -- {}",
                    timestamp,
                    record.level(),
                    message
                ));
            })
            .chain(log_file)
            .level(log::LevelFilter::Off)
            .level_for("panic", log::LevelFilter::Error.min(file_level_filter))
            .level_for(
                "iced_wgpu",
                log::LevelFilter::Info.min(file_level_filter),
            )
            .level_for("data", file_level_filter)
            .level_for("ipc", file_level_filter)
            .level_for("halloy", file_level_filter)
    })
}

fn pane_dispatch(
    env_rust_log: Option<log::Level>,
) -> (fern::Dispatch, ReceiverStream<Vec<Record>>) {
    // Set filter to trace in order to perform filtering when receiving for logs
    // pane so that filter-level can be changed without restarting the
    // application
    let channel_level_filter = env_rust_log
        .map_or(log::LevelFilter::Trace, |env| env.to_level_filter());

    let (channel_sink, receiver) = channel_logger();

    (
        fern::Dispatch::new()
            .chain(channel_sink)
            .level(log::LevelFilter::Off)
            .level_for(
                "panic",
                log::LevelFilter::Error.min(channel_level_filter),
            )
            .level_for(
                "iced_wgpu",
                log::LevelFilter::Info.min(channel_level_filter),
            )
            .level_for("data", channel_level_filter)
            .level_for("ipc", channel_level_filter)
            .level_for("halloy", channel_level_filter),
        receiver,
    )
}

fn channel_logger() -> (Box<dyn Log>, ReceiverStream<Vec<Record>>) {
    struct Sink {
        sender: mpsc::Sender<Record>,
    }

    impl Log for Sink {
        fn enabled(&self, _metadata: &::log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &::log::Record) {
            let _ = self.sender.send(Record {
                timestamp: Utc::now(),
                level: record.level().into(),
                message: format!("{}", record.args()),
            });
        }

        fn flush(&self) {}
    }

    let (log_sender, log_receiver) = mpsc::channel();
    let (async_sender, async_receiver) = tokio_mpsc::channel(1);

    thread::spawn(move || {
        const BATCH_SIZE: usize = 25;
        const BATCH_TIMEOUT: Duration = Duration::from_millis(250);

        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut timeout = Instant::now();

        loop {
            if let Ok(log) = log_receiver.recv_timeout(BATCH_TIMEOUT) {
                batch.push(log);
            }

            if batch.len() >= BATCH_SIZE
                || (!batch.is_empty() && timeout.elapsed() >= BATCH_TIMEOUT)
            {
                timeout = Instant::now();

                let _ = async_sender.blocking_send(mem::replace(
                    &mut batch,
                    Vec::with_capacity(BATCH_SIZE),
                ));
            }
        }
    });

    (
        Box::new(Sink { sender: log_sender }),
        ReceiverStream::new(async_receiver),
    )
}
