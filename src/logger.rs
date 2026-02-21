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
    is_debug: bool,
    config: config::Logs,
) -> Result<ReceiverStream<Vec<Record>>, Error> {
    let env_rust_log = env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::parse::<log::Level>)
        .transpose()?;

    let file_level_filter = env_rust_log
        .map_or(log::LevelFilter::from(config.file_level), |env| {
            env.to_level_filter()
        });

    let channel_level_filter = env_rust_log
        .map_or(log::LevelFilter::Trace, |env| env.to_level_filter());

    let mut io_sink = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "{}:{} -- {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            record.level(),
            message
        ));
    });

    if is_debug {
        io_sink = io_sink.chain(std::io::stdout());
    } else {
        let log_file = data::log::file()?;

        io_sink = io_sink.chain(log_file);
    }

    io_sink = io_sink
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("data", file_level_filter)
        .level_for("ipc", file_level_filter)
        .level_for("halloy", file_level_filter);

    let (channel_sink, receiver) = channel_logger();

    let channel_sink = fern::Dispatch::new()
        .chain(channel_sink)
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("data", channel_level_filter)
        .level_for("ipc", channel_level_filter)
        .level_for("halloy", channel_level_filter);

    fern::Dispatch::new()
        .chain(io_sink)
        .chain(channel_sink)
        .apply()?;

    Ok(receiver)
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
