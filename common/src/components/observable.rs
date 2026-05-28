use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc, sync::atomic::AtomicUsize};

/// A simple observable value that can be observed for changes.
pub type Observer<T> = dyn Fn(&T) + 'static;

/// A unique identifier for an observer.
pub type ObserverId = usize;

/// A simple observable that can be observed for notifications.
pub trait Observable<T> {
    /// Adds an observer that will be called on subject notification.
    fn observe(&mut self, observer: impl Fn(&T) + 'static) -> ObserverId;

    /// Removes an observer by its unique identifier.
    fn unobserve(&mut self, id: ObserverId) -> bool;
}

/// A simple subject that can be observed for notifications.
pub struct Subject<T> {
    observers: RefCell<HashMap<ObserverId, Rc<Observer<T>>>>,
    id_gen: AtomicUsize,
}

impl<T> fmt::Debug for Subject<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subject")
            .field("observers", &self.observers.borrow().len())
            .finish()
    }
}

impl<T> Observable<T> for Subject<T> {
    fn observe(&mut self, observer: impl Fn(&T) + 'static) -> ObserverId {
        let id = self
            .id_gen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.observers.borrow_mut().insert(id, Rc::new(observer));
        id
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.observers.borrow_mut().remove(&id).is_some()
    }
}

impl<T> Subject<T> {
    /// Creates a new observable subject.
    pub fn new() -> Self {
        Self {
            observers: RefCell::new(HashMap::new()),
            id_gen: AtomicUsize::new(0),
        }
    }

    /// Notifies all observers with the given value.
    pub fn notify(&self, value: &T) {
        // Allow the observers to mutate the observers list while we are notifying them.
        let observers = self
            .observers
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for observer in observers.into_iter() {
            observer(value);
        }
    }
}

/// An observable value that can be observed for changes.
pub trait ObservableValue<T>: Observable<T> {
    /// Gets a reference to the current value.
    fn get(&self) -> &T;
}

/// A simple value that can be observed for changes.
pub struct SubjectValue<T> {
    value: T,
    subject: Subject<T>,
}

impl<T> fmt::Debug for SubjectValue<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectValue")
            .field("value", &self.value)
            .field("observers", &self.subject.observers.borrow().len())
            .finish()
    }
}

impl<T> Observable<T> for SubjectValue<T> {
    fn observe(&mut self, observer: impl Fn(&T) + 'static) -> ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}

impl<T> ObservableValue<T> for SubjectValue<T> {
    fn get(&self) -> &T {
        &self.value
    }
}

impl<T> SubjectValue<T> {
    /// Creates a new observable value with the given initial value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            subject: Subject::new(),
        }
    }

    /// Sets the value and notifies all observers of the change.
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.subject.notify(&self.value);
    }
}
