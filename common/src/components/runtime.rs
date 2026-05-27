use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
}

/// A simple runtime that can execute tasks sequentially.
pub struct Runtime {
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
            task();
        }
    }
}
