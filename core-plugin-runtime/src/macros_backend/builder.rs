use std::collections::HashMap;

use crate::{
    MylifePlugin,
    metadata::{ConfigItem, ConfigType, Member, MemberType, PluginMetadata, PluginUsage, Type},
    runtime::MylifePluginRuntime,
};

use super::{
    ActionRuntimeExecutor, ConfigRuntimeSetter, PluginRuntimeAccess, PluginRuntimeImpl,
    StateRuntime, StateRuntimeGetter, StateRuntimeRegister,
};

pub struct PluginRuntimeBuilder<PluginType: MylifePlugin + 'static> {
    name: Option<String>,
    module: Option<String>,
    usage: Option<PluginUsage>,
    version: Option<String>,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
    config_runtime: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    state_runtime: HashMap<String, StateRuntime<PluginType>>,
    action_runtime: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType: MylifePlugin + 'static> PluginRuntimeBuilder<PluginType> {
    pub fn new() -> Self {
        PluginRuntimeBuilder {
            name: None,
            module: None,
            usage: None,
            version: None,
            description: None,
            members: HashMap::new(),
            config: HashMap::new(),
            config_runtime: HashMap::new(),
            state_runtime: HashMap::new(),
            action_runtime: HashMap::new(),
        }
    }

    pub fn build(self) -> Box<dyn MylifePluginRuntime> {
        let generator_panic = "Plugin macros error: missing field, this indicates an incorrect behavior in the macro code generator";

        PluginRuntimeImpl::<PluginType>::new(
            PluginMetadata::new(
                self.name.expect(generator_panic),
                self.module.expect(generator_panic),
                self.usage.expect(generator_panic),
                self.version.expect(generator_panic),
                self.description,
                self.members,
                self.config,
            ),
            PluginRuntimeAccess::new(self.config_runtime, self.state_runtime, self.action_runtime),
        )
    }

    pub fn set_plugin(
        &mut self,
        name: &str,
        description: Option<&str>,
        usage: PluginUsage,
        package_name: &str,
        package_version: &str,
    ) {
        let module_name = {
            use convert_case::{Case, Casing};

            let formatted = package_name.to_case(Case::Kebab);
            String::from(formatted.trim_start_matches("plugin-"))
        };

        self.name = Some(String::from(name));
        self.module = Some(module_name);
        self.usage = Some(usage);
        self.version = Some(String::from(package_version));
        self.description = description.map(String::from);
    }

    pub fn add_config(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: ConfigType,
        setter: ConfigRuntimeSetter<PluginType>,
    ) {
        let config_item = ConfigItem::new(description.map(String::from), value_type);
        self.config.insert(String::from(name), config_item);
        self.config_runtime.insert(String::from(name), setter);
    }

    pub fn add_state(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        register: StateRuntimeRegister<PluginType>,
        getter: StateRuntimeGetter<PluginType>,
    ) {
        let member = Member::new(description.map(String::from), MemberType::State, value_type);
        self.members.insert(String::from(name), member);
        self.state_runtime
            .insert(String::from(name), StateRuntime { register, getter });
    }

    pub fn add_action(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        executor: ActionRuntimeExecutor<PluginType>,
    ) {
        let member = Member::new(
            description.map(String::from),
            MemberType::Action,
            value_type,
        );
        self.members.insert(String::from(name), member);
        self.action_runtime.insert(String::from(name), executor);
    }
}

pub type BuilderPartCallback<PluginType> = fn(builder: &mut PluginRuntimeBuilder<PluginType>);

#[macro_export]
macro_rules! publish_plugin {
    ($plugin_type:ty) => {
        inventory::submit!(core_plugin_runtime::PluginRegistration::new::<$plugin_type>());
    };
}

pub use publish_plugin;
