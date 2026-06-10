use std::time::Duration;

use tokio::time::sleep;

use common::{
    bus::{self, Transport},
    components::{Components, ShutdownMessage},
};
use plugin_runtime::runtime::{Config, ConfigValue, Value};

use crate::components::{LocalComponents, LocalPlugins};

mod components;
mod modules;

mod modules_include {
    #![allow(unused_imports)]

    use plugin_logic_base::*;
}

const INSTANCE_NAME: &str = "core-test-instance";
const SERVER_ADDRESS: &str = "rpi-dev-home-main:1883";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    modules::init();

    let mut components = Components::new();
    let components_sender = components.get_mailbox_handle();
    components.add_handler(LocalPlugins::new());
    components.add_handler(LocalComponents::new(components_sender.clone()));
    let components_handle = components.start();

    let mut transport = Transport::new(INSTANCE_NAME.to_owned(), SERVER_ADDRESS.to_owned())?;
    let bus_sender = transport.get_mailbox_handle();
    let transport_handle = transport.start();

    sleep(Duration::from_secs(10)).await;
    println!("Will shutdown");

    bus_sender.send(Box::new(bus::ShutdownMessage));
    components_sender.send(Box::new(ShutdownMessage));

    let (components_res, transport_res) = tokio::join!(components_handle, transport_handle);
    components_res.expect("failed to join components");
    transport_res.expect("failed to join transport");

    Ok(())
}

#[allow(dead_code)]
fn old_main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    modules::init();

    let mut component = modules::registry()
        .plugin("logic-base.value-binary")
        .unwrap()
        .create(
            "comp-id",
            Box::new(|| {
                println!("WAKE ASKED");
            }),
            Box::new(|name, value| {
                println!("STATE CHANGE: {} -> {:?}", name, value);
            }),
        );

    let mut config = Config::new();
    config.insert("config".to_string(), ConfigValue::Bool(false));

    println!("configure");
    component.configure(&config)?;

    println!("init");
    component.init()?;
    println!("after init: state = {:?}", component.get_state("state"));

    println!("execute_action on");
    component.execute_action("on", Value::Bool(true))?;

    println!("execute_action off");
    component.execute_action("off", Value::Bool(true))?;

    Ok(())
}
