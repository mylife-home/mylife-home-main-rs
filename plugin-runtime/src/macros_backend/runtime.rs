use std::marker::PhantomData;

use crate::{
    metadata::{ConfigType, PluginMetadata, Type},
    runtime::{MyLifePluginRuntime, MylifeComponent, Value},
};

pub struct MyLifePluginRuntimeImpl<Component: MylifeComponent + Default + 'static> {
    metadata: PluginMetadata,
    _marker: PhantomData<Component>, // only to keep Component type info
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntimeImpl<Component> {
    pub fn new(metadata: PluginMetadata) -> Box<Self> {
        Box::new(MyLifePluginRuntimeImpl {
            metadata,
            _marker: PhantomData,
        })
    }
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntime
    for MyLifePluginRuntimeImpl<Component>
{
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create(&self) -> Box<dyn MylifeComponent> {
        Box::new(Component::default())
    }
}

//-----

impl Definition {
    pub const fn new_config(
        name: &'static str,
        description: Option<&'static str>,
        r#type: ConfigType,
        setter: fn(arg: &Value),
    ) -> Self {
        Definition::Config(ConfigDefinition {
            name,
            description,
            r#type,
            setter,
        })
    }

    pub const fn new_state(
        name: &'static str,
        description: Option<&'static str>,
        r#type: Type,
    ) -> Self {
        Definition::State(StateDefinition {
            name,
            description,
            r#type,
        })
    }

    pub const fn new_action(
        name: &'static str,
        description: Option<&'static str>,
        r#type: Type,
    ) -> Self {
        Definition::Action(ActionDefinition {
            name,
            description,
            r#type,
        })
    }
}

pub enum Definition {
    Config(ConfigDefinition),
    State(StateDefinition),
    Action(ActionDefinition),
}

pub struct ConfigDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: ConfigType,
    setter: fn(arg: &Value),
}

pub struct StateDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: Type,
}

pub struct ActionDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: Type,
}
