use std::alloc::System;

use module::Module;
use plugin_runtime::runtime::{Config, ConfigValue, Value};

mod module;

#[global_allocator]
static ALLOCATOR: System = System;

// TODO: logs
// TODO: anyhow::Error ?

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugins = Module::load("target/debug", "logic_base")?;

    for plugin in plugins.iter() {
        println!("Plugin loaded: {} v{}", plugin.id(), plugin.version());
        println!("{:?}", plugin.metadata());
    }

    let mut component = plugins[0].create_component();

    component.set_on_fail(Box::new(|error: Box<dyn std::error::Error>| {
        println!("FAIL: {}", error);
    }));

    component.set_on_state(Box::new(|name: &str, value: Value| {
        println!("STATE: {} = {:?}", name, value);
    }));

    let mut config = Config::new();
    config.insert("config".to_string(), ConfigValue::Bool(false));

    println!("configure");
    component.configure(&config);

    println!("init");
    component.init();
    println!(
        "after init: state = {:?}",
        component.get_state("state").expect("could not get state")
    );

    println!("execute_action on");
    component.execute_action("on", Value::Bool(true));

    println!("execute_action off");
    component.execute_action("off", Value::Bool(true));

    component.execute_action("toggle", Value::Bool(true));

    Ok(())
}
