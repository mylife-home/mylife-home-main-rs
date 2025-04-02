use crate::runtime::MylifePluginRuntime;

use crate::MylifePlugin;

pub struct PluginRegistration(fn() -> Box<dyn MylifePluginRuntime>);

impl PluginRegistration {
    pub const fn new<PluginType: MylifePlugin>() -> Self {
        PluginRegistration(|| PluginType::runtime())
    }

    pub fn runtimes() -> Vec<Box<dyn MylifePluginRuntime>> {
        inventory::iter::<PluginRegistration>()
            .map(|item| (item.0)())
            .collect()
    }
}

inventory::collect!(PluginRegistration);
