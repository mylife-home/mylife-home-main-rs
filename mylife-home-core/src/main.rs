use std::alloc::System;

use module::Module;

mod module;

#[global_allocator]
static ALLOCATOR: System = System;

// TODO: logs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugins = Module::load("target/debug", "logic_base")?;

    for plugin in plugins.iter() {
        println!("Plugin loaded: {} v{}", plugin.id(), plugin.version());
    }

    Ok(())
}
