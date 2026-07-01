use crate::runtime;
use common::components::{
    metadata,
    types::{TypedInto, Value},
};

use std::{fmt, sync::Arc};

/// MylifePluginHooks defines the lifecycle hooks a plugin author implements.
/// These are the author-facing entry points, wrapped by the runtime machinery.
pub trait MylifePluginHooks: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Creates a new instance with the given component id.
    fn new(id: &str, waker: WakeHandle) -> Self;

    /// Starts the instance once its config has been applied. Called after
    /// configuration, before any action is handled.
    fn init(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Hook to drive the instance's asynchronous work (network, timers, ...)
    /// outside of synchronous action handling.
    fn async_handler(&mut self) {}
}

/// MylifePlugin is implemented by the plugin type itself, on top of the
/// instance hooks, to expose the runtime used to register and instantiate it.
pub trait MylifePlugin: MylifePluginHooks + fmt::Debug + Send + Sync {
    /// Builds the runtime descriptor used to export this plugin.
    fn runtime() -> Box<dyn runtime::MylifePluginRuntime>;
}

/// Binding between a state field and the actor: the listener forwards changes
/// out, and the type tells how to convert the typed value to a Value.
struct StateRuntimeData {
    listener: Box<dyn Fn(Value) + Send + Sync>,
    r#type: metadata::Type,
}

impl fmt::Debug for StateRuntimeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateRuntimeData")
            .field("listener", &"Box<dyn Fn(Value)>")
            .field("type", &self.r#type)
            .finish()
    }
}

/// State is a typed state member of a component. It holds the current value
/// and, once bound, forwards every change to the actor via its listener.
#[derive(Debug)]
pub struct State<T: Default> {
    value: T,
    runtime: Option<StateRuntimeData>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        State {
            value: T::default(),
            runtime: None,
        }
    }
}

impl<T: Default + Clone + TypedInto<Value>> State<T> {
    /// Sets the value and notifies the bound listener. Panics if the state
    /// has not been bound to the runtime yet.
    pub fn set(&mut self, value: T) {
        let Some(StateRuntimeData { listener, r#type }) = self.runtime.as_ref() else {
            // stat not bound, trigger during configure() or init()
            return;
        };

        self.value = value;
        let value = self.value.clone().typed_into(r#type);
        listener(value);
    }

    /// Returns a reference to the current value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Binds the state to the runtime, installing the listener and the type
    /// used to convert outgoing values. Called once during setup.
    pub fn runtime_register(
        &mut self,
        listener: Box<dyn Fn(Value) + Send + Sync>,
        r#type: metadata::Type,
    ) {
        self.runtime = Some(StateRuntimeData { listener, r#type });
    }
}

/// WakeHandle lets a component ask the actor to run its async work. Calling
/// `wake()` posts a message to the components actor, which later runs the
/// component's async_handler from within its own task. It does not run the
/// work directly, so it is safe to call from any thread (a background task,
/// a timer, ...). Cheap to clone; hand a clone to whatever needs to wake the
/// component.
#[derive(Clone)]
pub struct WakeHandle {
    runtime: Arc<dyn Fn() + Send + Sync>,
}

impl fmt::Debug for WakeHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WakeHandle").finish_non_exhaustive()
    }
}

impl WakeHandle {
    /// Creates a handle from the callback that posts the wake message to the
    /// components actor. Built by the runtime when a component is set up.
    pub(crate) fn new(runtime: Box<dyn Fn() + Send + Sync>) -> Self {
        Self {
            runtime: Arc::new(runtime),
        }
    }

    /// Schedules the component's async_handler to run on the actor task.
    /// Returns immediately; the work happens later, in order, on that task.
    pub fn wake(&self) {
        (self.runtime)();
    }
}
