use std::collections::HashMap;
use std::sync::Arc;

use super::ids::{Id, IdError};

/// Shorthand for "the error the key type's project fails with".
type KeyError<K> = <K as Id>::Error;

/// A persistent map from a typed id to a shared value.
///
/// Every mutation returns a new collection and leaves the receiver alone, so
/// an operation can build its result and still fail afterwards without having
/// damaged anything. Cloning is one `Arc` bump, and an untouched collection
/// keeps its pointer, which is what lets a project be rebuilt per operation
/// and still only pay for the parts that moved.
///
/// The key type is part of the collection's type, so a `WindowId` cannot be
/// used to look into the resources; and a key is valid by construction, so
/// `set` has no id to check.
#[derive(Debug)]
pub struct Collection<K: Id, V>(Arc<HashMap<K, Arc<V>>>);

// Derived Clone would demand `V: Clone`, which is never needed: cloning a
// collection clones one Arc.
impl<K: Id, V> Clone for Collection<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K: Id, V> Default for Collection<K, V> {
    fn default() -> Self {
        Self(Arc::new(HashMap::new()))
    }
}

impl<K: Id, V> Collection<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, id: &K) -> bool {
        self.0.contains_key(id)
    }

    pub fn get(&self, id: &K) -> Option<&Arc<V>> {
        self.0.get(id)
    }

    pub fn get_required(&self, id: &K) -> Result<&Arc<V>, K::Error> {
        self.0
            .get(id)
            .ok_or_else(|| KeyError::<K>::not_found(K::KIND, id.as_str()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &Arc<V>)> {
        self.0.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Arc<V>> {
        self.0.values()
    }

    /// Insert or replace. No id check: `K` is already valid by construction.
    pub fn set(&self, id: K, value: Arc<V>) -> Self {
        let mut items = self.0.as_ref().clone();
        items.insert(id, value);
        Self(Arc::new(items))
    }

    /// Insert, failing if the id is taken.
    pub fn inserted(&self, id: K, value: Arc<V>) -> Result<Self, K::Error> {
        if self.0.contains_key(&id) {
            return Err(KeyError::<K>::already_exists(K::KIND, id.as_str()));
        }

        Ok(self.set(id, value))
    }

    /// Replace, failing if the id is absent.
    pub fn updated(&self, id: K, value: Arc<V>) -> Result<Self, K::Error> {
        if !self.0.contains_key(&id) {
            return Err(KeyError::<K>::not_found(K::KIND, id.as_str()));
        }

        Ok(self.set(id, value))
    }

    /// `None` when the id was not there, so callers can report "nothing
    /// changed" without a second lookup.
    pub fn removed(&self, id: &K) -> Option<Self> {
        let mut items = self.0.as_ref().clone();
        items.remove(id)?;
        Some(Self(Arc::new(items)))
    }

    /// `None` when old and new id are equal, `Err` when the new id is taken or
    /// the old one is missing.
    pub fn renamed(&self, id: &K, new_id: &K) -> Result<Option<Self>, K::Error> {
        if id == new_id {
            return Ok(None);
        }

        if self.0.contains_key(new_id) {
            return Err(KeyError::<K>::already_exists(K::KIND, new_id.as_str()));
        }

        let mut items = self.0.as_ref().clone();
        let value = items
            .remove(id)
            .ok_or_else(|| KeyError::<K>::not_found(K::KIND, id.as_str()))?;
        items.insert(new_id.clone(), value);
        Ok(Some(Self(Arc::new(items))))
    }

    /// Rewrite the values that the closure changes. `None` when nothing moved,
    /// so an unchanged subtree keeps its pointer and callers can skip emitting.
    pub fn map_values(
        &self,
        mut update: impl FnMut(&K, &Arc<V>) -> Option<Arc<V>>,
    ) -> Option<Self> {
        let mut items: Option<HashMap<K, Arc<V>>> = None;

        for (key, value) in self.0.iter() {
            let Some(updated) = update(key, value) else {
                continue;
            };

            items
                .get_or_insert_with(|| self.0.as_ref().clone())
                .insert(key.clone(), updated);
        }

        items.map(|items| Self(Arc::new(items)))
    }
}

impl<K: Id, V> Collection<K, V> {
    /// Keep only the entries the predicate accepts. `None` when none were
    /// dropped, so an untouched collection keeps its pointer.
    pub fn retained(&self, mut keep: impl FnMut(&K, &Arc<V>) -> bool) -> Option<Self> {
        let mut items = self.0.as_ref().clone();
        let before = items.len();
        items.retain(|key, value| keep(key, value));

        (items.len() != before).then(|| Self(Arc::new(items)))
    }
}

impl<K: Id, V> FromIterator<(K, Arc<V>)> for Collection<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, Arc<V>)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl<'a, K: Id, V> IntoIterator for &'a Collection<K, V> {
    type Item = (&'a K, &'a Arc<V>);
    type IntoIter = std::collections::hash_map::Iter<'a, K, Arc<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// ---------------------------------------------------------------------------
// Wire maps
//
// Both projects store their maps as `Arc<HashMap<String, Arc<T>>>` on the wire,
// so the two conversions are domain-independent and live here.
// ---------------------------------------------------------------------------

/// The wire's `Arc<HashMap<String, Arc<W>>>` back to a `Collection<K, V>`.
///
/// This one is a real impl because `Self` is ours. Both the key and the value
/// convert, so one bound covers every map on the wire.
impl<K, V, W> TryFrom<&HashMap<String, Arc<W>>> for Collection<K, V>
where
    K: Id + TryFrom<String, Error = <K as Id>::Error>,
    V: for<'a> TryFrom<&'a W, Error = <K as Id>::Error>,
{
    type Error = <K as Id>::Error;

    fn try_from(value: &HashMap<String, Arc<W>>) -> Result<Self, Self::Error> {
        value
            .iter()
            .map(|(id, value)| {
                Ok((
                    K::try_from(id.clone())?,
                    Arc::new(V::try_from(value.as_ref())?),
                ))
            })
            .collect()
    }
}

impl<K: Id, V> Collection<K, V> {
    /// The other direction, and the one place the orphan rule actually bites.
    ///
    /// `impl<K, V, W> From<&Collection<K, V>> for Arc<HashMap<String, Arc<W>>>`
    /// is rejected: `Self` is built entirely from foreign type constructors and
    /// carries the uncovered parameter `W`, which sits ahead of the first local
    /// type in the impl. RFC 2451 allows a local type in a foreign trait's
    /// parameters, not a naked type parameter in front of one.
    ///
    /// Writing it per concrete pair (`From<&Collection<WindowId, Window>> for
    /// Arc<HashMap<String, Arc<UiWindowData>>>`, and six more) would be legal,
    /// because then there is no type parameter at all. Seven identical impls
    /// delegating to one body is worse than a method, so this is a method.
    pub fn to_wire<W>(&self) -> Arc<HashMap<String, Arc<W>>>
    where
        W: for<'a> From<&'a V>,
    {
        Arc::new(
            self.iter()
                .map(|(id, value)| (id.to_string(), Arc::new(W::from(value.as_ref()))))
                .collect(),
        )
    }
}
