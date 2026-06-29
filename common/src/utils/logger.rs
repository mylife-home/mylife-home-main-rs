use log::{LevelFilter, Log, Metadata, Record};
use std::{
    env,
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

/// A log::Log that fans each record out to a dynamic set of child loggers.
/// Children can be added after the logger is installed, so an early console
/// logger can be in place at boot and the MQTT logger added once the bus is up.
pub struct MultiLogger {
    loggers: RwLock<Vec<(u64, Box<dyn Log>)>>,
    next_id: AtomicU64,
}

static LOGGER: MultiLogger = MultiLogger::new();

/// Installs the global logger. Call once, early. Children are added separately.
pub fn init(console: bool) {
    log::set_logger(&LOGGER).expect("logger already initialized");
    log::set_max_level(LevelFilter::Trace);

    if console {
        let mut builder = pretty_env_logger::formatted_builder();

        if let Ok(s) = env::var("RUST_LOG") {
            builder.parse_filters(&s);
        }

        add_logger(Box::new(builder.build()));
    }
}

/// Adds a child logger and returns its id. Safe to call after `init`, from any thread.
pub fn add_logger(logger: Box<dyn Log>) -> u64 {
    let id = LOGGER.next_id.fetch_add(1, Ordering::Relaxed);
    LOGGER
        .loggers
        .write()
        .expect("could not acquire write lock")
        .push((id, logger));
    id
}

/// Removes a previously-added child by id, flushing it before dropping.
pub fn remove_logger(id: u64) {
    let mut loggers = LOGGER
        .loggers
        .write()
        .expect("could not acquire write lock");

    if let Some(pos) = loggers.iter().position(|(lid, _)| *lid == id) {
        let (_, logger) = loggers.remove(pos);
        logger.flush();
    }
}

impl MultiLogger {
    const fn new() -> Self {
        Self {
            loggers: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(0),
        }
    }
}

impl Log for MultiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.loggers
            .read()
            .expect("could not acquire read lock")
            .iter()
            .any(|(_, logger)| logger.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        self.loggers
            .read()
            .expect("could not acquire read lock")
            .iter()
            .for_each(|(_, logger)| logger.log(record));
    }

    fn flush(&self) {
        self.loggers
            .read()
            .expect("could not acquire read lock")
            .iter()
            .for_each(|(_, logger)| logger.flush());
    }
}
