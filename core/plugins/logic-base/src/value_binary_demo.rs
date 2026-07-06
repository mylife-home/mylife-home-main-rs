use std::{
    convert::Infallible,
    fmt, mem,
    sync::{Arc, Mutex, MutexGuard},
};

use plugin_macros::{MylifePlugin, mylife_actions};
use plugin_runtime::{MylifePluginHooks, State, WakeHandle};

#[derive(MylifePlugin, Debug)]
#[mylife_plugin(description = "step relay", usage = "logic")] // name=
pub struct ValueBinaryDemo {
    id: String,

    #[mylife_config(description = "initial value (useless only config example")] // type=, name=
    config: bool,

    #[mylife_state(description = "actual value")] // type=, name=
    state: State<bool>,

    updates: Arc<UpdateSync<ValueBinaryDemo>>,
}

// impl Drop if terminate needed
impl MylifePluginHooks for ValueBinaryDemo {
    type Error = Infallible;

    fn new(id: &str, waker: WakeHandle) -> Self {
        ValueBinaryDemo {
            id: String::from(id),
            config: Default::default(),
            state: Default::default(),
            updates: Arc::new(UpdateSync::new(waker)),
        }
    }

    fn init(&mut self) -> Result<(), Self::Error> {
        self.state.set(self.config);

        tracing::debug!(
            component_id = self.id,
            state = self.state.get(),
            "initial state"
        );

        Ok(())
    }

    fn async_handler(&mut self) {
        let updates = self.updates.clone();
        updates.run(self);
    }
}

#[mylife_actions]
impl ValueBinaryDemo {
    // can return Result<(), Self::Error> or nothing
    #[mylife_action(description = "set value to on")] // type=, name=
    fn on(&mut self, arg: bool) -> Result<(), Infallible> {
        if arg {
            //self.state.set(true);
            // showcase async work
            tokio::spawn({
                let updates = self.updates.clone();
                async move {
                    updates.enqueue({
                        move |comp: &mut Self| {
                            comp.state.set(true);
                        }
                    });
                }
            });
        }

        Ok(())
    }

    #[mylife_action(description = "set value to off")]
    fn off(&mut self, arg: bool) {
        if arg {
            self.state.set(false);
        }
    }

    #[mylife_action(description = "toggle value")]
    fn toggle(&mut self, arg: bool) {
        if arg {
            self.state.set(!self.state.get());
        }
    }
}

struct UpdateSync<ComponentType> {
    queue: Mutex<Vec<Box<dyn FnOnce(&mut ComponentType) + Send>>>,
    waker: WakeHandle,
}

impl<ComponentType> fmt::Debug for UpdateSync<ComponentType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateSync")
            .field("queue", &self.queue().len())
            .field("waker", &self.waker)
            .finish()
    }
}

impl<ComponentType> UpdateSync<ComponentType> {
    pub fn new(waker: WakeHandle) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            waker,
        }
    }

    fn queue(&self) -> MutexGuard<'_, Vec<Box<dyn FnOnce(&mut ComponentType) + Send>>> {
        self.queue.lock().expect("cannot lock mutex")
    }

    pub fn enqueue<Update: FnOnce(&mut ComponentType) + Send + 'static>(&self, update: Update) {
        self.queue().push(Box::new(update));
        self.waker.wake();
    }

    pub fn run(&self, component: &mut ComponentType) {
        let queue = {
            let mut storage = Vec::new();
            let mut current = self.queue();
            mem::swap(&mut storage, &mut current);
            storage
        };

        for update in queue {
            update(component);
        }
    }
}
