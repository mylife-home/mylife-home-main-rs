use std::{
    fmt,
    sync::{Arc, Mutex},
};

use log::warn;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
};

/// An unbounded, single-consumer mailbox for messages of type `Message`.
///
/// The owning actor holds the `Mailbox` and is the sole receiver; producers
/// hold cheap, cloneable [`MailboxHandle`]s obtained from [`Mailbox::handle`].
pub struct Mailbox<Message: fmt::Debug> {
    sender: UnboundedSender<Message>,
    receiver: UnboundedReceiver<Message>,
}

impl<Message: fmt::Debug> Mailbox<Message> {
    /// Creates an empty mailbox with no pending messages.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self { sender, receiver }
    }

    /// Returns a cloneable handle for sending messages into this mailbox.
    /// Hand these to producers (other tasks, background work, ...).
    pub fn handle(&self) -> MailboxHandle<Message> {
        MailboxHandle {
            sender: self.sender.clone(),
        }
    }

    /// Receives the next message, waiting until one is available.
    pub async fn recv(&mut self) -> Message {
        // Note: channel cannot be closed since we have a sender instance
        self.receiver
            .recv()
            .await
            .expect("unexpected closed channel")
    }
}

/// A cheap, cloneable sender into a [`Mailbox`]. Obtained via
/// [`Mailbox::handle`] and shared freely with any producer.
pub struct MailboxHandle<Message: fmt::Debug> {
    sender: UnboundedSender<Message>,
}

impl<T: fmt::Debug> Clone for MailboxHandle<T> {
    fn clone(&self) -> Self {
        MailboxHandle {
            sender: self.sender.clone(),
        }
    }
}

impl<Message: fmt::Debug> MailboxHandle<Message> {
    /// Sends a message to the mailbox. Non-blocking and never waits.
    ///
    /// A send can only fail if the receiving [`Mailbox`] has been dropped; in
    /// that case the message is logged and discarded rather than propagated.
    pub fn send(&self, msg: Message) {
        if let Err(e) = self.sender.send(msg) {
            log::error!("could not send message {:?} to mailbox: {}", e.0, e);
        }
    }
}

/// Shareable reply handle for actor RPC messages.
///
/// Unlike a raw [`oneshot::Sender`], this can be cloned and passed to multiple handlers by
/// reference — whichever handler calls [`send`](ReplySender::send) first delivers the reply.
#[derive(Debug, Clone)]
pub struct ReplySender<Reply: fmt::Debug>(Arc<Mutex<Option<oneshot::Sender<Reply>>>>);

impl<Reply: fmt::Debug> ReplySender<Reply> {
    /// Creates a new `ReplySender` and its paired [`oneshot::Receiver`] for the caller to await.
    pub fn create_channel() -> (Self, oneshot::Receiver<Reply>) {
        let (sender, receiver) = oneshot::channel();
        let reply = Self(Arc::new(Mutex::new(Some(sender))));

        (reply, receiver)
    }

    /// Sends the reply. Only the first call has any effect; subsequent calls are ignored with a warning.
    pub fn send(&self, msg: Reply) {
        if let Some(sender) = self.0.lock().expect("could not lock mutex").take() {
            if let Err(msg) = sender.send(msg) {
                warn!("Could not send reply '{:?}'", msg);
            }
        } else {
            warn!("Tried to send multiple reply '{:?}', ignored", msg);
        }
    }

    /// Returns `true` if the reply has already been sent.
    pub fn is_sent(&self) -> bool {
        self.0.lock().expect("could not lock mutex").is_none()
    }
}
