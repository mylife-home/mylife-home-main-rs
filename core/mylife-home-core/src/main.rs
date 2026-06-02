use std::time::Duration;

use tokio::{sync::mpsc, time::sleep};

use common::{
    bus::Transport,
    components::{ComponentChange, Components},
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

    let (component_sender, components_mailbox) = mpsc::unbounded_channel();
    let mut components = Components::new(components_mailbox);
    components.add_handler(Extension::new());
    components.start();

    let (bus_sender, bus_mailbox) = mpsc::unbounded_channel();
    let mut transport = Transport::new(
        bus_mailbox,
        INSTANCE_NAME.to_owned(),
        SERVER_ADDRESS.to_owned(),
    )?;
    transport.start();

    sleep(Duration::from_secs(10)).await;

    let _ = component_sender;
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
