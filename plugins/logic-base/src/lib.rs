mod value_binary;

use plugin_runtime::{export_plugin, PluginRegistry};
use value_binary::ValueBinary;

export_plugin!(register);

extern "C" fn register(registry: &mut dyn PluginRegistry) {
    registry.register_plugin(ValueBinary::runtime());
}
