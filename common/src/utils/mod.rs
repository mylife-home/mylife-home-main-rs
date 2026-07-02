use std::{fs, io};

use tokio::signal::unix::{SignalKind, signal};

pub mod actors;
pub mod config;
pub mod logger;

pub fn hostname() -> io::Result<String> {
    Ok(fs::read_to_string("/proc/sys/kernel/hostname")?
        .trim_end()
        .to_owned())
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
    #[serde(default)]
    pub logger_output_console: bool,
    pub kameo_console_listen_address: Option<String>,
}