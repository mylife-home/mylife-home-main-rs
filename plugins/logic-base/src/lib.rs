mod value_binary;

use core_plugin_runtime::{export_module, MylifePlugin, PluginRegistry};
use value_binary::ValueBinary;

export_module!(register);

fn register(registry: &mut dyn PluginRegistry) {
    registry.register_plugin(ValueBinary::runtime());
}
