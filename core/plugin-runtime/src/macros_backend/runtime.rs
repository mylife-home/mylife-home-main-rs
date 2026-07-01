use std::{collections::HashMap, fmt, sync::Arc};

use crate::{
    MylifePlugin, WakeHandle,
    runtime::{ConfigError, MylifeComponent, MylifePluginRuntime, PluginError},
};
use common::components::{
    metadata::PluginMetadata,
    types::{Config, ConfigValue, Value},
};

/// PluginRuntimeImpl is the concrete runtime for a given plugin type. It pairs
/// the plugin metadata with the typed accessors used to drive instances, and
/// acts as the factory that produces components.
#[derive(Debug)]
pub struct PluginRuntimeImpl<PluginType: MylifePlugin + 'static> {
    metadata: Arc<PluginMetadata>,
    access: Arc<PluginRuntimeAccess<PluginType>>,
}

impl<PluginType: MylifePlugin + 'static> PluginRuntimeImpl<PluginType> {
    /// Creates a runtime from the plugin metadata and its typed accessors.
    pub fn new(
        metadata: PluginMetadata,
        access: Arc<PluginRuntimeAccess<PluginType>>,
    ) -> Box<Self> {
        Box::new(PluginRuntimeImpl {
            metadata: Arc::new(metadata),
            access,
        })
    }
}

impl<PluginType: MylifePlugin> MylifePluginRuntime for PluginRuntimeImpl<PluginType> {
    fn metadata(&self) -> &Arc<PluginMetadata> {
        &self.metadata
    }

    fn create(
        &self,
        id: &str,
        waker: Box<dyn Fn() + Send + Sync>,
        state_change: Box<dyn Fn(/*name:*/ &str, /*value:*/ &Value) + Send + Sync>,
    ) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(
            &self.access,
            id,
            self.metadata.clone(),
            waker,
            state_change,
        )
    }
}

/// StateRuntime holds the typed accessors for a single state member: how to
/// register its change listener and how to read its current value.
#[derive(Debug)]
pub struct StateRuntime<PluginType> {
    pub(crate) register: StateRuntimeRegister<PluginType>,
    pub(crate) getter: StateRuntimeGetter<PluginType>,
}

/// Applies a config value to the plugin instance.
pub type ConfigRuntimeSetter<PluginType> =
    fn(target: &mut PluginType, config: ConfigValue) -> Result<(), ConfigError>;
/// Installs the listener invoked whenever a state member changes.
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn Fn(Value) + Send + Sync>) -> ();
/// Reads the current value of a state member.
pub type StateRuntimeGetter<PluginType> = fn(target: &PluginType) -> Value;
/// Executes an action on the plugin instance.
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: Value) -> Result<(), PluginError>;

/// PluginRuntimeAccess is the generated dispatch table for a plugin type: the
/// typed setters, getters, and executors that bridge the erased Value world to
/// the plugin's concrete fields and methods. Shared across all its instances.
#[derive(Debug)]
pub struct PluginRuntimeAccess<PluginType: MylifePlugin> {
    configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    states: HashMap<String, StateRuntime<PluginType>>,
    actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType: MylifePlugin> PluginRuntimeAccess<PluginType> {
    /// Builds the access table from the per-member accessor maps.
    pub fn new(
        configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
        states: HashMap<String, StateRuntime<PluginType>>,
        actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
    ) -> Arc<Self> {
        Arc::new(PluginRuntimeAccess {
            configs,
            states,
            actions,
        })
    }
}

/// ComponentImpl is a live plugin instance: it wraps the user plugin value,
/// holds its id and metadata, and exposes the Component interface by routing
/// through the shared access table.
struct ComponentImpl<PluginType: MylifePlugin> {
    access: Arc<PluginRuntimeAccess<PluginType>>,
    component: PluginType,
    id: String,
    plugin_metadata: Arc<PluginMetadata>,
    state_change: Arc<dyn Fn(/*name:*/ &str, /*value:*/ &Value) + Send + Sync>,
}

impl<PluginType: MylifePlugin> fmt::Debug for ComponentImpl<PluginType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("ComponentImpl")
            .field("access", &self.access)
            .field("component", &self.component)
            .field("id", &self.id)
            .finish()
    }
}

impl<PluginType: MylifePlugin> ComponentImpl<PluginType> {
    /// Creates an instance, builds the plugin value, and wires its state.
    pub fn new(
        access: &Arc<PluginRuntimeAccess<PluginType>>,
        id: &str,
        plugin_metadata: Arc<PluginMetadata>,
        waker: Box<dyn Fn() + Send + Sync>,
        state_change: Box<dyn Fn(/*name:*/ &str, /*value:*/ &Value) + Send + Sync>,
    ) -> Box<Self> {
        Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::new(id, WakeHandle::new(waker)),
            id: String::from(id),
            plugin_metadata,
            state_change: Arc::new(state_change),
        })
    }

    /// Installs, for each state member, a listener that forwards its changes.
    fn register_state_handlers(&mut self) {
        for (name, state) in self.access.states.iter() {
            let id = self.id.clone();
            let name = name.clone();
            let state_change = self.state_change.clone();
            (state.register)(
                &mut self.component,
                Box::new(move |value: Value| {
                    tracing::trace!(component_id = id, state = name, ?value, "state changed");
                    state_change(&name, &value);
                }),
            );
        }
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn id(&self) -> &str {
        &self.id
    }

    fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin_metadata
    }

    fn configure(&mut self, config: &Config) -> Result<(), ConfigError> {
        tracing::trace!(component_id = self.id, ?config, "configure");

        for (name, setter) in self.access.configs.iter() {
            let value = config
                .get(name)
                .ok_or_else(|| ConfigError::key_not_found(name))?
                .clone();

            setter(&mut self.component, value)?;
        }

        Ok(())
    }

    fn init(&mut self) -> Result<(), PluginError> {
        self.component.init().map_err(PluginError::new)?;

        // Register after init, so that state changes are not trigger before.
        // The component actor will then publish all state at once.
        // This avoid spurious triggers that may happen during configure()/init()
        self.register_state_handlers();

        Ok(())
    }

    fn async_handler(&mut self) {
        self.component.async_handler();
    }

    fn get_state(&self, name: &str) -> Value {
        let state = self
            .access
            .states
            .get(name)
            .unwrap_or_else(|| panic!("state '{}' not found on component '{}'", name, self.id));
        (state.getter)(&self.component)
    }

    fn execute_action(&mut self, name: &str, value: Value) -> Result<(), PluginError> {
        let handler =
            self.access.actions.get(name).unwrap_or_else(|| {
                panic!("action '{}' not found on component '{}'", name, self.id)
            });

        tracing::trace!(
            component_id = self.id,
            action = name,
            ?value,
            "execute action"
        );

        handler(&mut self.component, value)
    }
}
