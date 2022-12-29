mod value_binary;

use plugin_runtime::{export_module, MylifePlugin, PluginRegistry};
use value_binary::ValueBinary;

export_module!(register);

extern "Rust" fn register(registry: &mut dyn PluginRegistry) {
    registry.register_plugin(ValueBinary::runtime());
}
