use std::{alloc::System, sync::Arc, io};
use libloading::Library;
use plugin_runtime::PluginDeclaration;

#[global_allocator]
static ALLOCATOR: System = System;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    unsafe {
        let library = Arc::new(Library::new("target/debug/liblogic_base.so")?);

        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

            if decl.rustc_version != plugin_runtime::RUSTC_VERSION
            || decl.core_version != plugin_runtime::CORE_VERSION
            || decl.mylife_runtime_version != plugin_runtime::MYLIFE_RUNTIME_VERSION
        {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Runtime version mismatch",
            )));
        }

        println!("plugin version: {}", decl.plugin_version);
    }; 

    Ok(())
}
