use std::{
    env, mem,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use log::Log;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub use data::log::{Error, Record};

pub fn setup(is_debug: bool) -> Result<ReceiverStream<Vec<Record>>, Error> {
    let level_filter = env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::parse::<log::Level>)
        .transpose()?
        .unwrap_or(log::Level::Debug)
        .to_level_filter();

    let mut io_sink = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "{}:{} -- {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            record.level(),
            message
        ))
    });

    if is_debug {
        io_sink = io_sink.chain(std::io::stdout());
    } else {
        let log_file = data::log::file()?;

        io_sink = io_sink.chain(log_file);
    }

    let (channel_sink, reciever) = channel_logger();

    fern::Dispatch::new()
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("data", level_filter)
        .level_for("halloy", level_filter)
        .chain(io_sink)
        .chain(channel_sink)
        .apply()?;

    Ok(reciever)
}

fn channel_logger() -> (Box<dyn Log>, ReceiverStream<Vec<Record>>) {
    let (log_sender, log_receiver) = mpsc::channel();
    let (async_sender, async_receiver) = tokio_mpsc::channel(1);

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

                let _ = async_sender
                    .blocking_send(mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE)));
            }
        }
    });

    (
        Box::new(Sink { sender: log_sender }),
        ReceiverStream::new(async_receiver),
    )
}
