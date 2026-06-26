use serde::{Deserialize, Serialize};
use serde_with::{TimestampSeconds, serde_as};
use std::{collections::HashMap, time::SystemTime};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    /// 'ui' | 'studio' | 'core' | 'driver? (for arduino/esp/...)'
    pub r#type: String,

    /// main: Raspberry ... | nodemcu | x64
    /// others are details like ram, cpu, ...
    pub hardware: HashMap<String, String>,

    /// rpi
    /// - os: linux-xxx
    /// - node: 24.5
    /// - mylife-home-core: 1.0.0
    /// - mylife-home-common: 1.0.0
    ///
    /// esp/arduino
    /// - mylife: 1.21.4
    pub versions: HashMap<String, String>,

    #[serde_as(as = "TimestampSeconds<i64>")]
    pub system_uptime: SystemTime,
    #[serde_as(as = "TimestampSeconds<i64>")]
    pub instance_uptime: SystemTime,
    pub hostname: String,
    pub capabilities: Vec<String>,

    pub wifi: Option<Wifi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wifi {
    pub rssi: i64,
}
