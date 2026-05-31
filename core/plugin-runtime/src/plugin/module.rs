use crate::runtime::MylifePluginRuntime;

use crate::MylifePlugin;

/// PluginRegistration is a compile-time registration entry for a plugin,
/// collected via `inventory` so plugins declare themselves without a central
/// list. It holds a constructor for the plugin's runtime descriptor.
pub struct PluginRegistration(fn() -> Box<dyn MylifePluginRuntime>);

impl PluginRegistration {
    /// Creates a registration for the given plugin type. Use with
    /// `inventory::submit!` so the plugin is discovered at startup.
    pub const fn new<PluginType: MylifePlugin>() -> Self {
        PluginRegistration(|| PluginType::runtime())
    }

    /// Builds the runtime descriptor of every registered plugin.
    pub fn runtimes() -> Vec<Box<dyn MylifePluginRuntime>> {
        inventory::iter::<PluginRegistration>()
            .map(|item| (item.0)())
            .collect()
    }
}

inventory::collect!(PluginRegistration);
