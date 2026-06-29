use std::{
    fmt,
    sync::{
        Arc, LazyLock, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

/// A consumer of fanned-out log events (MQTT forwarder, syslog, ...).
/// `emit` is called synchronously from the layer, so impls must not block:
/// the MQTT sink sends to the bus mailbox and returns.
pub trait LogSink: Send + Sync {
    fn emit(&self, event: &LogEvent);
    fn flush(&self) {}
}

/// A typed structured-log value, preserving the type tracing captured rather
/// than stringifying it. Sinks decide how to render or serialize each variant.
#[derive(Clone, Debug)]
pub enum LogValue {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    /// Also fallback for types captured only via Debug (errors, custom types, &c.).
    Str(String),
}

/// Owned, library-neutral form of an event, built once and shared with every sink.
#[derive(Clone)]
pub struct LogEvent {
    pub level: tracing::Level,
    pub target: String,
    pub fields: Vec<(String, LogValue)>,
}

struct Sinks {
    list: RwLock<Vec<(u64, Box<dyn LogSink>)>>,
    next_id: AtomicU64,
}

/// Shared sink registry. The fan-out layer holds one clone; the free functions
/// reach it through this static, so add/remove work after install, from any thread.
static SINKS: LazyLock<Arc<Sinks>> = LazyLock::new(|| {
    Arc::new(Sinks {
        list: RwLock::new(Vec::new()),
        next_id: AtomicU64::new(0),
    })
});

/// Installs the global subscriber. Call once, early. Sinks are added separately.
pub fn init(console: bool) {
    let fanout = FanoutLayer {
        sinks: SINKS.clone(),
    };

    let registry = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env()) // honors RUST_LOG
        .with(fanout);

    // Console is its own fmt layer: proper formatting (level, target, fields) for free.
    if console {
        registry.with(tracing_subscriber::fmt::layer()).init();
    } else {
        registry.init();
    }
}

/// Adds a sink and returns its id. Safe to call after `init`, from any thread.
pub fn add_logger(sink: Box<dyn LogSink>) -> u64 {
    let id = SINKS.next_id.fetch_add(1, Ordering::Relaxed);
    SINKS
        .list
        .write()
        .expect("could not acquire write lock")
        .push((id, sink));
    id
}

/// Removes a sink by id, flushing it before dropping.
pub fn remove_logger(id: u64) {
    let mut list = SINKS.list.write().expect("could not acquire write lock");
    if let Some(pos) = list.iter().position(|(sid, _)| *sid == id) {
        let (_, sink) = list.remove(pos);
        sink.flush();
    }
}

struct FanoutLayer {
    sinks: Arc<Sinks>,
}

impl<S: Subscriber> Layer<S> for FanoutLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let meta = event.metadata();
        let log_event = LogEvent {
            level: *meta.level(),
            target: meta.target().to_owned(),
            fields: visitor.fields,
        };

        for (_, sink) in self
            .sinks
            .list
            .read()
            .expect("could not acquire read lock")
            .iter()
        {
            sink.emit(&log_event);
        }
    }
}

/// Captures the message and the remaining structured fields with their types intact.
#[derive(Default)]
struct FieldVisitor {
    fields: Vec<(String, LogValue)>,
}

impl FieldVisitor {
    fn push(&mut self, field: &Field, value: LogValue) {
        self.fields.push((field.name().to_owned(), value));
    }
}

impl Visit for FieldVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, LogValue::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, LogValue::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, LogValue::U64(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push(field, LogValue::F64(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, LogValue::Str(value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.push(field, LogValue::Str(format!("{:?}", value)));
    }
}
