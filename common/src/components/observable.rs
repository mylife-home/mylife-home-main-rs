use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc, sync::atomic::AtomicUsize};

/// A simple observable value that can be observed for changes.
pub type Observer<T> = dyn Fn(&T) + 'static;

/// A unique identifier for an observer.
pub type ObserverId = usize;

/// A simple value that can be observed for changes.
pub struct Value<T> {
    value: T,
    observers: RefCell<HashMap<ObserverId, Rc<Observer<T>>>>,
    id_gen: AtomicUsize,
}

impl<T> fmt::Debug for Value<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value")
            .field("value", &self.value)
            .field("observers", &self.observers.borrow().len())
            .finish()
    }
}

impl<T> Value<T> {
    /// Creates a new observable value with the given initial value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            observers: RefCell::new(HashMap::new()),
            id_gen: AtomicUsize::new(0),
        }
    }

    /// Gets a reference to the current value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Sets the value and notifies all observers of the change.
    pub fn set(&mut self, value: T) {
        self.value = value;
        // Allow the observers to mutate the observers list while we are notifying them.
        let observers = self
            .observers
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for observer in observers.into_iter() {
            observer(&self.value);
        }
    }

    /// Adds an observer that will be called whenever the value changes.
    pub fn observe(&mut self, observer: impl Fn(&T) + 'static) -> ObserverId {
        let id = self
            .id_gen
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.observers.borrow_mut().insert(id, Rc::new(observer));
        id
    }

    /// Removes an observer by its unique identifier.
    pub fn unobserve(&mut self, id: ObserverId) -> bool {
        self.observers.borrow_mut().remove(&id).is_some()
    }
}
