use std::{cell::RefCell, sync::Mutex};

use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// A simple runtime that can execute tasks sequentially.
pub type Task = Box<dyn FnOnce() + Send + 'static>;

/// A sender for tasks to be executed by the runtime.
#[derive(Clone)]
pub struct TaskSender(UnboundedSender<Task>);

impl TaskSender {
    /// Executes a task in the runtime.
    pub fn execute(&self, task: Task) {
        self.0.send(task).expect("failed to send task");
    }

    /// Runs a task immediately if we are on the runtime thread, or sends it to the runtime otherwise.
    pub fn run_or_send(&self, task: Task) {
        if TaskSender::is_runtime_thread() {
            task();
        } else {
            self.execute(task);
        }
    }

    /// Checks if the current thread is the runtime thread.
    pub fn is_runtime_thread() -> bool {
        IS_RUNTIME_THREAD.with(|flag| *flag.borrow())
    }
}

/// A simple runtime that can execute tasks sequentially.
struct Runtime {
    rx: UnboundedReceiver<Task>,
}

impl Runtime {
    /// Creates a new runtime and starts its worker task.
    pub fn create() -> (Self, TaskSender) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Self { rx }, TaskSender(tx))
    }

    /// Runs the runtime and executes tasks until the channel is closed.
    pub async fn run(mut self) {
        while let Some(task) = self.rx.recv().await {
            let _guard = RuntimeGuard::new();
            task();
        }
    }
}

static RUNTIME_HANDLE: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Initializes the runtime and returns a sender for tasks to be executed by the runtime.
pub fn init() -> TaskSender {
    let mut handle = RUNTIME_HANDLE
        .lock()
        .expect("Failed to acquire runtime handle lock");
    if handle.is_some() {
        panic!("Runtime is already initialized");
    }

    let (runtime, sender) = Runtime::create();
    *handle = Some(tokio::spawn(runtime.run()));
    sender
}

/// Waits for the runtime to finish executing all tasks.
pub async fn wait_exit() {
    let Some(handle) = RUNTIME_HANDLE
        .lock()
        .expect("Failed to acquire runtime handle lock")
        .take()
    else {
        panic!("Runtime is not initialized");
    };

    handle.await.expect("Failed to wait for runtime to finish");
}

struct RuntimeGuard;

impl RuntimeGuard {
    pub fn new() -> Self {
        IS_RUNTIME_THREAD.with(|flag| {
            if *flag.borrow() {
                panic!("RuntimeGuard can only be created once per thread");
            }
            *flag.borrow_mut() = true;
        });

        Self
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        IS_RUNTIME_THREAD.with(|flag| {
            *flag.borrow_mut() = false;
        });
    }
}

thread_local! {
    static IS_RUNTIME_THREAD: RefCell<bool> = RefCell::new(false);
}
