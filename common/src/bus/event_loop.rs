use futures::{
    stream::{BoxStream, SelectAll},
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};

/// A callback function to be executed by the event loop.
pub type Callback = Box<dyn FnOnce() + Send + 'static>;

/// A source of events for the event loop.
pub type Source = BoxStream<'static, Callback>;

/// An event loop that can execute callbacks and manage event sources.
pub struct EventLoop {
    executor: mpsc::UnboundedSender<Callback>,
    source: mpsc::UnboundedSender<Source>,
    exit: oneshot::Sender<()>,
    worker: tokio::task::JoinHandle<()>,
}

impl EventLoop {
    /// Creates a new event loop and starts its worker task.
    pub fn create() -> Self {
        let (exit, exit_rx) = oneshot::channel();
        let (executor, executor_rx) = mpsc::unbounded_channel();
        let (source, source_rx) = mpsc::unbounded_channel();

        let worker = tokio::spawn(async move {
            Self::run(exit_rx, executor_rx, source_rx).await;
        });

        Self {
            executor,
            source,
            exit,
            worker,
        }
    }

    /// Terminates the event loop and waits for the worker task to finish.
    pub async fn terminate(self) {
        self.exit.send(()).expect("failed to send termination signal");
        self.worker.await.expect("failed to join event loop worker");
    }

    /// Registers a new event source with the event loop.
    pub fn register_source(&self, source: Source) {
        self.source
            .send(source)
            .expect("failed to register event loop source");
    }

    /// Executes a callback function in the event loop.
    pub fn execute(&self, callback: Callback) {
        self.executor
            .send(callback)
            .expect("failed to send callback");
    }

    async fn run(
        mut exit: oneshot::Receiver<()>,
        executor: mpsc::UnboundedReceiver<Callback>,
        mut source: mpsc::UnboundedReceiver<Source>,
    ) {
        let mut stream = SelectAll::<Source>::new();
        stream.push(Box::pin(UnboundedReceiverStream::new(executor)));

        loop {
            tokio::select! {
                _ = &mut exit => {
                    break;
                }
                Some(source) = source.recv() => {
                    stream.push(source);
                }
                callback = stream.next() => {
                    let callback = callback.expect("event loop stream ended");
                    callback();
                }
            }
        }
    }
}
