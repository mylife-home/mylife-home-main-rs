use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use bytes::Bytes;
use serde::{de::DeserializeOwned, ser::Serialize};
use thiserror::Error;
use tokio::fs;

/// Represents an event that occurs within an `FsCollection`.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub origin: Origin,
    pub kind: Kind,
}

/// Represents the type of change that occurred to an item in the `FsCollection`.
#[derive(Debug, Clone)]
pub enum Kind {
    Created,
    Updated,
    Deleted,
    Renamed { new_id: String },
}

/// Represents the origin of a change to an item in the `FsCollection`.
#[derive(Debug, Clone)]
pub enum Origin {
    Internal,
    External,
}

/// Errors that can occur when working with an `FsCollection`.
#[derive(Error, Debug)]
pub enum FsCollectionError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Item already exists: {0}")]
    AlreadyExists(String),
}

/// A wrapper struct that holds a value along with a list of events associated with it.
pub struct WithEvents<T> {
    value: T,
    events: Vec<Event>,
}

impl<T> WithEvents<T> {
    /// Creates a new `WithEvents` instance with the given value and a list of events.
    pub fn multi(value: T, events: Vec<Event>) -> Self {
        Self { value, events }
    }

    /// Creates a new `WithEvents` instance with the given value and a single event.
    pub fn single(value: T, event: Event) -> Self {
        Self {
            value,
            events: vec![event],
        }
    }

    /// Creates a new `WithEvents` instance with the given value and no events.
    pub fn empty(value: T) -> Self {
        Self {
            value,
            events: Vec::new(),
        }
    }
}

impl<T> From<WithEvents<T>> for (T, Vec<Event>) {
    fn from(with_events: WithEvents<T>) -> Self {
        (with_events.value, with_events.events)
    }
}

#[derive(Debug)]
pub struct FsCollection<T: DeserializeOwned + Serialize> {
    items: HashMap<String, Item<T>>,
    path: PathBuf,
}

impl<T: DeserializeOwned + Serialize> FsCollection<T> {
    /// Creates a new `FsCollection` instance with the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            items: HashMap::new(),
            path,
        }
    }

    /// Refreshes the collection with regard to the current state of the filesystem.
    pub async fn refresh(&mut self) -> WithEvents<()> {
        let mut id_set = self.items.keys().cloned().collect::<HashSet<_>>();
        let mut events = Vec::new();

        let mut readdir = match fs::read_dir(&self.path).await {
            Ok(rd) => rd,
            Err(e) => {
                tracing::error!(error = ?e, "refresh: failed to read directory");
                return WithEvents::empty(());
            }
        };

        loop {
            let entry = match readdir.next_entry().await {
                Ok(Some(e)) => e,
                Ok(None) => break,
                Err(e) => {
                    tracing::error!(error = ?e, "refresh: error reading directory entry");
                    break;
                }
            };

            // get id from the file name (remove .json)
            let file_name = entry.file_name();

            let Some(file_name_str) = file_name.to_str() else {
                tracing::warn!("Failed to convert file name to string: {:?}", file_name);
                continue;
            };

            let Some(id) = file_name_str.strip_suffix(".json") else {
                tracing::debug!(
                    "Failed to strip .json suffix from file name: {}",
                    file_name_str
                );
                continue;
            };

            let maybe_existing_item = self.items.get_mut(id);
            if let Some(item) = maybe_existing_item {
                let updated = match item.handle_refresh().await {
                    Ok(updated) => updated,
                    Err(e) => {
                        tracing::error!(error = ?e, "refresh: error handling item refresh");
                        continue;
                    }
                };

                if updated {
                    events.push(Event {
                        id: id.to_owned(),
                        kind: Kind::Updated,
                        origin: Origin::External,
                    });
                }

                id_set.remove(id);
            } else {
                let item = match Item::handle_new(self.make_path(id)).await {
                    Ok(item) => item,
                    Err(e) => {
                        tracing::error!(error = ?e, "refresh: error handling new item");
                        continue;
                    }
                };

                self.items.insert(id.to_owned(), item);

                events.push(Event {
                    id: id.to_owned(),
                    kind: Kind::Created,
                    origin: Origin::External,
                });
            }
        }

        // Remove items that are no longer present in the filesystem.
        for id in id_set {
            self.items.remove(&id);
            events.push(Event {
                id,
                kind: Kind::Deleted,
                origin: Origin::External,
            });
        }

        WithEvents::multi((), events)
    }

    /// Sets the value of an item in the collection, creating it if it does not exist.
    pub async fn set(&mut self, id: &str, value: T) -> Result<WithEvents<()>, FsCollectionError> {
        if self.items.contains_key(id) {
            self.update(id, value).await
        } else {
            self.create(id, value).await
        }
    }

    /// Creates a new item in the collection with the given ID and value. Fails if the item already exists.
    pub async fn create(
        &mut self,
        id: &str,
        value: T,
    ) -> Result<WithEvents<()>, FsCollectionError> {
        if self.items.contains_key(id) {
            return Err(FsCollectionError::AlreadyExists(id.to_owned()));
        }

        let path = self.make_path(id);
        let item = Item::new(path, value).await?;

        self.items.insert(id.to_owned(), item);

        Ok(WithEvents::single(
            (),
            Event {
                id: id.to_owned(),
                kind: Kind::Created,
                origin: Origin::Internal,
            },
        ))
    }

    /// Updates the value of an existing item in the collection. Fails if the item does not exist.
    pub async fn update(
        &mut self,
        id: &str,
        value: T,
    ) -> Result<WithEvents<()>, FsCollectionError> {
        let item = self
            .items
            .get_mut(id)
            .ok_or_else(|| FsCollectionError::NotFound(id.to_owned()))?;
        item.update(value).await?;

        Ok(WithEvents::single(
            (),
            Event {
                id: id.to_owned(),
                kind: Kind::Updated,
                origin: Origin::Internal,
            },
        ))
    }

    /// Renames an existing item in the collection. Fails if the item does not exist.
    pub async fn rename(
        &mut self,
        id: &str,
        new_id: &str,
    ) -> Result<WithEvents<()>, FsCollectionError> {
        if self.items.contains_key(new_id) {
            return Err(FsCollectionError::AlreadyExists(new_id.to_owned()));
        }
        let new_path = self.make_path(new_id);

        // do not remove it before real rename
        let item = self
            .items
            .get_mut(id)
            .ok_or_else(|| FsCollectionError::NotFound(id.to_owned()))?;
        item.rename(new_path).await?;

        let item = self.items.remove(id).expect("item not found");
        self.items.insert(new_id.to_owned(), item);

        Ok(WithEvents::single(
            (),
            Event {
                id: id.to_owned(),
                kind: Kind::Renamed {
                    new_id: new_id.to_owned(),
                },
                origin: Origin::Internal,
            },
        ))
    }

    /// Deletes an existing item from the collection. Fails if the item does not exist.
    pub async fn delete(&mut self, id: &str) -> Result<WithEvents<()>, FsCollectionError> {
        // do not remove it before real rename
        let item = self
            .items
            .get_mut(id)
            .ok_or_else(|| FsCollectionError::NotFound(id.to_owned()))?;

        item.delete().await?;
        self.items.remove(id);

        Ok(WithEvents::single(
            (),
            Event {
                id: id.to_owned(),
                kind: Kind::Deleted,
                origin: Origin::Internal,
            },
        ))
    }

    /// Retrieves the value of an existing item in the collection. Fails if the item does not exist.
    pub async fn get(&self, id: &str) -> Result<&T, FsCollectionError> {
        let item = self
            .items
            .get(id)
            .ok_or_else(|| FsCollectionError::NotFound(id.to_owned()))?;
        Ok(&item.value)
    }

    fn make_path(&self, id: &str) -> PathBuf {
        self.path.join(id.to_string() + ".json")
    }
}

#[derive(Debug)]
struct Item<T: DeserializeOwned + Serialize> {
    path: PathBuf,
    modified: SystemTime,
    size: u64,
    raw: Bytes,
    value: T,
}

impl<T: DeserializeOwned + Serialize> Item<T> {
    /// Creates a new `Item` from the given value, writing it to the specified path.
    pub async fn new(path: PathBuf, value: T) -> Result<Self, FsCollectionError> {
        let raw = Bytes::from_owner(serde_json::to_vec(&value)?);

        fs::write(&path, &raw).await?;

        let metadata = fs::metadata(&path).await?;
        let size = metadata.len();
        let modified = metadata.modified()?;

        Ok(Self {
            path,
            modified,
            size,
            raw,
            value,
        })
    }

    /// Updates the item with the new value, writing it to the file and updating its metadata.
    pub async fn update(&mut self, value: T) -> Result<(), FsCollectionError> {
        let raw = Bytes::from_owner(serde_json::to_vec(&value)?);
        fs::write(&self.path, &raw).await?;

        let metadata = fs::metadata(&self.path).await?;
        let modified = metadata.modified()?;
        let size = metadata.len();

        self.modified = modified;
        self.size = size;
        self.raw = raw;
        self.value = value;

        Ok(())
    }

    /// Renames the item to the new path, updating its internal path field.
    pub async fn rename(&mut self, new_path: PathBuf) -> Result<(), FsCollectionError> {
        fs::rename(&self.path, &new_path).await?;

        self.path = new_path;

        Ok(())
    }

    /// Deletes the item from the filesystem.
    pub async fn delete(&self) -> Result<(), FsCollectionError> {
        fs::remove_file(&self.path).await?;
        Ok(())
    }

    /// Handle new item from filesystem.
    pub async fn handle_new(path: PathBuf) -> Result<Self, FsCollectionError> {
        let metadata = fs::metadata(&path).await?;
        let modified = metadata.modified()?;
        let size = metadata.len();
        let raw = Bytes::from_owner(fs::read(&path).await?);
        let value = serde_json::from_slice(&raw)?;

        Ok(Self {
            path,
            modified,
            size,
            raw,
            value,
        })
    }

    /// Handle rename from filesystem.
    pub fn handle_rename(&mut self, new_path: PathBuf) {
        self.path = new_path;
    }

    /// Handle refresh from filesystem
    pub async fn handle_refresh(&mut self) -> Result<bool, FsCollectionError> {
        let metadata = fs::metadata(&self.path).await?;

        let modified = metadata.modified()?;
        let size = metadata.len();

        if self.modified == modified && self.size == size {
            return Ok(false);
        }

        let raw = Bytes::from_owner(fs::read(&self.path).await?);
        if raw == self.raw {
            return Ok(false);
        }

        let value = serde_json::from_slice(&raw)?;

        // Update the item's fields
        self.modified = modified;
        self.size = size;
        self.raw = raw;
        self.value = value;

        Ok(true)
    }

    /// Returns a reference to the path of the item.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the value contained in the item.
    pub fn value(&self) -> &T {
        &self.value
    }
}
