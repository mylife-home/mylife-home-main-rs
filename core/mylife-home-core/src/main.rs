use std::time::Duration;

use tokio::{sync::mpsc, time::sleep};

use common::{
    bus::{self, Transport},
    components::{ComponentChange, Components, ShutdownMessage},
};
use plugin_runtime::runtime::{Config, ConfigValue, Value};

use crate::components::Extension;

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

    let (components_sender, components_mailbox) = mpsc::unbounded_channel();
    let mut components = Components::new(components_mailbox);
    components.add_handler(Extension::new());
    let components_handle = components.start();

    let (bus_sender, bus_mailbox) = mpsc::unbounded_channel();
    let mut transport = Transport::new(
        bus_mailbox,
        INSTANCE_NAME.to_owned(),
        SERVER_ADDRESS.to_owned(),
    )?;
    let transport_handle = transport.start();

    sleep(Duration::from_secs(10)).await;
    println!("Will shutdown");

    bus_sender
        .send(Box::new(bus::ShutdownMessage))
        .expect("could not send to bus");

    components_sender
        .send(Box::new(ShutdownMessage))
        .expect("could not send to components");

    let (components_res, transport_res) = tokio::join!(components_handle, transport_handle);
    components_res.expect("failed to join components");
    transport_res.expect("failed to join transport");

    let _ = components_sender;
    let _ = bus_sender;

    Ok(())
}

#[allow(dead_code)]
fn old_main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    modules::init();

    let mut component = modules::registry()
        .plugin("logic-base.value-binary")
        .unwrap()
        .create_component(
            "comp-id",
            Box::new(|| {
                println!("WAKE ASKED");
            }),
        );

    component.observe(Box::new(|event: &ComponentChange| {
        println!("EVENT: {:?}", event);
    }));

    let mut config = Config::new();
    config.insert("config".to_string(), ConfigValue::Bool(false));

    println!("configure");
    component.configure(&config)?;

    println!("init");
    component.init()?;
    println!(
        "after init: state = {:?}",
        component.get_state("state").expect("could not get state")
    );

    println!("execute_action on");
    component.execute_action("on", Value::Bool(true))?;

    println!("execute_action off");
    component.execute_action("off", Value::Bool(true))?;

    Ok(())
}
