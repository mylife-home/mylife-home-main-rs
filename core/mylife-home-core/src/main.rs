use std::alloc::System;

use plugin_runtime::runtime::{Config, ConfigValue, Value};

mod modules;

mod modules_include {
    use plugin_logic_base::*;
}

#[global_allocator]
static ALLOCATOR: System = System;

// TODO: Error: anyhow pour plugins, thiserror pour core ?
// TODO: try tokio with plugins (implement "minuterie")

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    modules::init();

    let mut component = modules::registry()
        .plugin("logic-base.value-binary")
        .unwrap()
        .create_component("comp-id");

    component.set_on_state(Box::new(|name: &str, value: Value| {
        println!("STATE: {} = {:?}", name, value);
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
