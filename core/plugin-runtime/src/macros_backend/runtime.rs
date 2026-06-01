use anyhow::Context;
use log::trace;
use std::{
    cell::UnsafeCell, collections::HashMap, fmt, marker::PhantomPinned, pin::Pin, sync::Arc,
};

use crate::{
    MylifePlugin, WakeHandle,
    runtime::{MylifeComponent, MylifePluginRuntime},
};
use common::{
    components::{
        Component, ComponentChange, ComponentChangeEventType,
        metadata::PluginMetadata,
        types::{Config, ConfigValue, Value},
    },
    utils::observable::{Observable, Observer, ObserverId, Subject},
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

    fn create(&self, id: &str, waker: Box<dyn Fn() + Send + Sync>) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(&self.access, id, waker, &self.metadata)
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
    fn(target: &mut PluginType, config: ConfigValue) -> anyhow::Result<()>;
/// Installs the listener invoked whenever a state member changes.
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn Fn(Value) + Send + Sync>) -> ();
/// Reads the current value of a state member.
pub type StateRuntimeGetter<PluginType> = fn(target: &PluginType) -> Value;
/// Executes an action on the plugin instance.
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: Value) -> anyhow::Result<()>;

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

// FIXME: remove Arc<Mutex<>>

/// ComponentImpl is a live plugin instance: it wraps the user plugin value,
/// holds its id and metadata, and exposes the Component interface by routing
/// through the shared access table. State changes are emitted via its subject.
struct ComponentImpl<PluginType: MylifePlugin> {
    access: Arc<PluginRuntimeAccess<PluginType>>,
    component: PluginType,
    id: String,
    plugin_metadata: Arc<PluginMetadata>,
    subject: SharedSubject,
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
    /// Creates an instance, builds the plugin value, and wires its state
    /// change listeners to the subject.
    pub fn new(
        access: &Arc<PluginRuntimeAccess<PluginType>>,
        id: &str,
        waker: Box<dyn Fn() + Send + Sync>,
        plugin_metadata: &Arc<PluginMetadata>,
    ) -> Box<Self> {
        let mut component = Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::new(id, WakeHandle::new(waker)),
            id: String::from(id),
            plugin_metadata: plugin_metadata.clone(),
            subject: SharedSubject::new(),
        });

        component.register_state_handlers();

        component
    }

    /// Installs, for each state member, a listener that forwards its changes
    /// to the subject as a ComponentChange::State event.
    fn register_state_handlers(&mut self) {
        for (name, state) in self.access.states.iter() {
            let id = self.id.clone();
            let name = name.clone();
            let subject_ref = self.subject.as_ref();
            (state.register)(
                &mut self.component,
                Box::new(move |value: Value| {
                    trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{id}] state '{name}' changed to {value:?}");
                    subject_ref.notify(&ComponentChange::State {
                        name: &name,
                        value: &value,
                    });
                }),
            );
        }
    }
}

impl<PluginType: MylifePlugin> Component for ComponentImpl<PluginType> {
    fn id(&self) -> &str {
        &self.id
    }

    fn plugin(&self) -> Arc<PluginMetadata> {
        self.plugin_metadata.clone()
    }

    fn get_state(&self, name: &str) -> Option<Value> {
        let state = self.access.states.get(name)?;
        Some((state.getter)(&self.component))
    }

    fn execute_action(&mut self, name: &str, action: Value) -> anyhow::Result<()> {
        let handler = self
            .access
            .actions
            .get(name)
            .with_context(|| format!("Action not found: {}", name))?;

        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] execute action '{name}' with {action:?}", self.id);
        handler(&mut self.component, action)
    }
}

impl<PluginType: MylifePlugin> Observable<ComponentChangeEventType> for ComponentImpl<PluginType> {
    fn observe(&mut self, observer: Box<Observer<ComponentChangeEventType>>) -> ObserverId {
        self.subject.get_mut().observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.get_mut().unobserve(id)
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn configure(&mut self, config: &Config) -> anyhow::Result<()> {
        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] configure with {config:?}", self.id);

        for (name, setter) in self.access.configs.iter() {
            let value = config
                .get(name)
                .context(format!("Config '{name}' not found"))?
                .clone();

            setter(&mut self.component, value)?;
        }

        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<()> {
        self.component.init()
    }

    fn async_handler(&mut self) {
        self.component.async_handler();
    }
}

/// SharedSubject
///
/// A component's state listeners are stored INSIDE the plugin, which lives
/// right next to the Subject inside ComponentImpl. A listener therefore cannot
/// hold a normal &mut to the Subject: that would be a second borrow of a
/// sibling field, and it must outlive the call that created it. So the listener
/// holds a pointer instead. SharedSubject is what makes that pointer sound, by
/// owning the two guarantees a raw pointer needs and nothing else:
///
///   1. Address stability. The Subject lives inside `Inner`, which is `!Unpin`
///      (PhantomPinned) and only ever exists as `Pin<Box<Inner>>`. No safe API
///      can move the value out of that box, so the Subject's address is fixed
///      for the whole life of the SharedSubject. This does NOT depend on how
///      ComponentImpl is built, moved, or stored: moving ComponentImpl moves
///      the Pin<Box> pointer, while `Inner` stays put at its heap address.
///
///   2. Sound aliasing. The Subject sits in an `UnsafeCell`. Every reference,
///      both the listener handle and the owner accessor, is derived from
///      `UnsafeCell::get()`, so they share a single provenance and may each
///      produce &mut. UnsafeCell is the one place the language allows this.
///
/// The obligation neither guarantee covers, that no two &mut are LIVE at the
/// same instant, is met by the runtime: all access happens on the single actor
/// task, synchronously, and never re-entrantly for one component.
struct SharedSubject {
    inner: Pin<Box<Inner>>,
}

struct Inner {
    cell: UnsafeCell<Subject<ComponentChangeEventType>>,
    // Makes Inner !Unpin so Pin<Box<Inner>> truly forbids moving it out.
    _pin: PhantomPinned,
}

impl SharedSubject {
    pub fn new() -> Self {
        SharedSubject {
            inner: Box::pin(Inner {
                cell: UnsafeCell::new(Subject::new()),
                _pin: PhantomPinned,
            }),
        }
    }

    /// A cheap Copy, Send + Sync handle for state listeners to capture.
    /// Valid for the whole life of this SharedSubject thanks to the pin.
    pub fn as_ref(&self) -> SharedSubjectRef {
        SharedSubjectRef(self.inner.as_ref().get_ref().cell.get())
    }

    /// Owner access for the Observable impl. Same provenance as the handle
    /// (both come from the cell). The returned &mut is short-lived and, on the
    /// single task, never coexists with a listener's &mut.
    pub fn get_mut(&self) -> &mut Subject<ComponentChangeEventType> {
        // SAFETY: see SharedSubject. Address-stable; single synchronous task;
        // non-reentrant per component, so no other &mut to the subject is live.
        unsafe { &mut *self.inner.as_ref().get_ref().cell.get() }
    }
}

/// A Copy, Send + Sync raw handle to a SharedSubject's Subject, captured by
/// state listeners. The unsafe Send + Sync are sound because the pointer is
/// only ever dereferenced on the single actor task (see SharedSubject); they
/// exist so the listener box keeps its `Box<dyn Fn(Value) + Send + Sync>`
/// type, which leaves the macro and plugin code untouched.
#[derive(Clone, Copy)]
struct SharedSubjectRef(*const Subject<ComponentChangeEventType>);

unsafe impl Send for SharedSubjectRef {}
unsafe impl Sync for SharedSubjectRef {}

impl SharedSubjectRef {
    /// Notify the shared subject. Must be called only on the actor task,
    /// synchronously, and not re-entrantly with another access to the same
    /// subject (the SharedSubject obligation).
    pub fn notify(self, change: &ComponentChange) {
        // SAFETY: see SharedSubject. No other &mut to the subject is live.
        unsafe { &*self.0 }.notify(change);
    }
}
