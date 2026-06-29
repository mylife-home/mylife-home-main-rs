use std::fs;

pub mod actors;
pub mod config;
pub mod logger;

pub fn hostname() -> anyhow::Result<String> {
    Ok(fs::read_to_string("/proc/sys/kernel/hostname")?
        .trim_end()
        .to_owned())
}
