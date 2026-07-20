use std::{fs, io, time::Duration};

use tokio::signal::unix::{SignalKind, signal};

pub mod actors;
pub mod config;
pub mod logger;

pub fn hostname() -> io::Result<String> {
    Ok(fs::read_to_string("/proc/sys/kernel/hostname")?
        .trim_end()
        .to_owned())
}

pub fn system_uptime() -> io::Result<Duration> {
    let content = fs::read_to_string("/proc/uptime")?;
    let seconds: f64 = content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad /proc/uptime format"))?;
    Ok(Duration::from_secs_f64(seconds))
}

pub async fn wait_for_shutdown_signal() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap(); // Ctrl+C
    let mut sigterm = signal(SignalKind::terminate()).unwrap(); // systemd stop

    tokio::select! {
        _ = sigint.recv()  => tracing::info!("received SIGINT, shutting down"),
        _ = sigterm.recv() => tracing::info!("received SIGTERM, shutting down"),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ObservabilityConfig {
    pub logger_level: Option<logger::ConfigLogLevel>,
    pub kameo_console_listen_address: Option<String>,
}
