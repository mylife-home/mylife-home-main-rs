use std::{
    collections::HashMap,
    format,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{SessionEvent, SessionEventType, SessionHandle, SessionId};

/// Represents an individual active notification subscription channel linked to a specific Session.
#[derive(Debug, Clone)]
pub struct Notifier<Data: serde::Serialize> {
    session: SessionHandle,
    notifier_type: String,
    notifier_id: String,
    _phantom: PhantomData<Data>,
}

impl<Data: serde::Serialize> Notifier<Data> {
    /// Create a new Notifier for a specific session, notifier type, and notifier ID.
    pub fn new(session: SessionHandle, notifier_type: impl Into<String>) -> Self {
        let notifier_id = NOTIFIER_ID_GENERATOR.generate_id();

        Self {
            session,
            notifier_type: notifier_type.into(),
            notifier_id,
            _phantom: PhantomData,
        }
    }

    /// Get the notifier ID.
    pub fn notifier_id(&self) -> &str {
        &self.notifier_id
    }

    /// Get the session ID associated with this notifier.
    pub fn session_id(&self) -> SessionId {
        self.session.id()
    }

    /// Notify the session with a notification message. The notification is sent to the session actor, which will handle it asynchronously.
    pub fn notify(&self, data: &Data) {
        self.session
            .notify(&self.notifier_type, &self.notifier_id, data);
    }
}

/// Manages all subscriptions across sessions for a specific domain (identified by featureName and notifierType).
#[derive(Debug)]
pub struct NotifierManager<Data: serde::Serialize> {
    notifier_type: String,
    notifiers: HashMap<String, Notifier<Data>>,
}

impl<Data: serde::Serialize> NotifierManager<Data> {
    /// Create a new NotifierManager.
    pub fn new(notifier_type: impl Into<String>) -> Self {
        Self {
            notifier_type: notifier_type.into(),
            notifiers: HashMap::new(),
        }
    }

    /// Handle session events.
    pub fn session_event(&mut self, event: &SessionEvent) {
        if event.event_type() == SessionEventType::Stopped {
            // Remove all notifiers associated with the stopped session.
            for (id, _) in self
                .notifiers
                .extract_if(|_, notifier| notifier.session == *event.session())
            {
                tracing::trace!(notifier = %id, session = %event.session().id(), "removing notifier for stopped session");
            }
        }
    }

    /// Create a new Notifier for a specific session and add it to the manager.
    pub fn create_notifier(&mut self, session: SessionHandle) -> &Notifier<Data> {
        let notifier = Notifier::new(session, &self.notifier_type);
        let id = notifier.notifier_id().to_string();
        self.notifiers.insert(id.clone(), notifier);
        self.notifiers
            .get(&id)
            .expect("notifier should exist after insertion")
    }

    /// Remove a Notifier from the manager by its ID.
    pub fn remove_notifier(&mut self, notifier_id: &str) {
        self.notifiers.remove(notifier_id);
    }

    /// Notify all active notifiers with the provided data.
    pub fn notify_all(&self, data: &Data) {
        for notifier in self.notifiers.values() {
            notifier.notify(data);
        }
    }
}

#[derive(Debug)]
struct NotifierIdGenerator(AtomicUsize);

impl NotifierIdGenerator {
    pub const fn new() -> Self {
        Self(AtomicUsize::new(1))
    }

    pub fn generate_id(&self) -> String {
        let id = self.0.fetch_add(1, Ordering::Relaxed);
        format!("{}", id)
    }
}

static NOTIFIER_ID_GENERATOR: NotifierIdGenerator = NotifierIdGenerator::new();
