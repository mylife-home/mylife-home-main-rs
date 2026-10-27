use std::{
    collections::{HashMap, HashSet},
    format, io, mem,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use common::utils::actors::{
    ActorHandle, CallError, HandleLookupError, PublisherHandle, SchedulerHandle, SpawnedActor,
    SpawnedActors, SubscriberHandle, spawn_pubsub,
};
use kameo::{message, prelude::*};

use bytes::Bytes;
use serde::{de::DeserializeOwned, ser::Serialize};
use thiserror::Error;
use tokio::fs;

/// Represents an event that occurs within an `FsCollection`.
#[derive(Debug)]
pub struct Update<T: DeserializeOwned + Serialize + Send + Sync + 'static> {
    pub id: String,
    pub origin: Origin,
    pub kind: Kind<T>,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> Clone for Update<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            origin: self.origin,
            kind: self.kind.clone(),
        }
    }
}

/// Represents the type of change that occurred to an item in the `FsCollection`.
#[derive(Debug)]
pub enum Kind<T: DeserializeOwned + Serialize + Send + Sync + 'static> {
    Created { value: Arc<T> },
    Updated { value: Arc<T> },
    Deleted,
    Renamed { new_id: String },
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> Clone for Kind<T> {
    fn clone(&self) -> Self {
        match self {
            Kind::Created { value } => Kind::Created {
                value: Arc::clone(value),
            },
            Kind::Updated { value } => Kind::Updated {
                value: Arc::clone(value),
            },
            Kind::Deleted => Kind::Deleted,
            Kind::Renamed { new_id } => Kind::Renamed {
                new_id: new_id.clone(),
            },
        }
    }
}

/// Represents the origin of a change to an item in the `FsCollection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A handle to drive a FsCollection actor
#[derive(Debug, Clone)]
pub struct FsCollectionHandle<T: DeserializeOwned + Serialize + Send + Sync + 'static> {
    actor: ActorHandle<FsCollection<T>>,
    on_update: SubscriberHandle<Update<T>>,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> FsCollectionHandle<T> {
    fn names(name: &str) -> (String, String) {
        let actor_name = format!("fscollection.{}", name);
        let pubsub_name = format!("fscollection.{}.update", name);
        (actor_name, pubsub_name)
    }

    /// Create a new access
    pub fn new(name: &str) -> Result<Self, HandleLookupError> {
        let (actor_name, pubsub_name) = Self::names(name);

        Ok(Self {
            actor: ActorHandle::from_name(actor_name)?,
            on_update: SubscriberHandle::from_name(pubsub_name)?,
        })
    }

    /// Starts the FsCollection actor monitoring.
    ///
    /// (reserved for start_collection)
    async fn start(&self) -> Result<(), CallError<StartError>> {
        self.actor.call(Start).await
    }

    /// Sets the value of an item in the collection, creating it if it does not exist.
    pub async fn set(&self, id: String, value: Arc<T>) -> Result<(), CallError<FsCollectionError>> {
        self.actor
            .call(Set {
                id,
                value,
                kind: SetKind::Set,
            })
            .await
    }

    /// Creates a new item in the collection with the given ID and value. Fails if the item already exists.
    pub async fn create(
        &self,
        id: String,
        value: Arc<T>,
    ) -> Result<(), CallError<FsCollectionError>> {
        self.actor
            .call(Set {
                id,
                value,
                kind: SetKind::Create,
            })
            .await
    }

    /// Updates the value of an existing item in the collection. Fails if the item does not exist.
    pub async fn update(
        &self,
        id: String,
        value: Arc<T>,
    ) -> Result<(), CallError<FsCollectionError>> {
        self.actor
            .call(Set {
                id,
                value,
                kind: SetKind::Update,
            })
            .await
    }

    /// Renames an existing item in the collection. Fails if the item does not exist.
    pub async fn rename(
        &self,
        id: String,
        new_id: String,
    ) -> Result<(), CallError<FsCollectionError>> {
        self.actor.call(Rename { id, new_id }).await
    }

    /// Deletes an existing item from the collection. Fails if the item does not exist.
    pub async fn delete(&self, id: String) -> Result<(), CallError<FsCollectionError>> {
        self.actor.call(Delete { id }).await
    }

    /// Retrieves the value of an existing item in the collection. Fails if the item does not exist.
    pub async fn get(&self, id: String) -> Result<Arc<T>, CallError<FsCollectionError>> {
        self.actor.call(Get { id }).await
    }

    /// Retrieves a list of all item IDs in the collection.
    pub async fn get_all(&self) -> Result<Vec<(String, Arc<T>)>, CallError<FsCollectionError>> {
        self.actor.call(GetAll).await
    }

    /// Returns a handle to subscribe to updates for this collection.
    pub fn on_update(&self) -> &SubscriberHandle<Update<T>> {
        &self.on_update
    }
}

/// Initializes a new collection with the given name and path.
pub async fn init_collection<T: DeserializeOwned + Serialize + Send + Sync + 'static>(
    actors: &mut SpawnedActors,
    name: &str,
    path: PathBuf,
) {
    let (actor_name, pubsub_name) = FsCollectionHandle::<T>::names(name);

    actors.add(spawn_pubsub::<Update<T>>(pubsub_name.clone()).await);

    let config = FsCollectionConfig { pubsub_name, path };
    let (fs_collection, _) = SpawnedActor::start::<FsCollection<T>>(config).await;

    fs_collection.register(actor_name);

    actors.add(fs_collection);
}

/// Starts an existing collection with the given name.
pub async fn start_collection<T: DeserializeOwned + Serialize + Send + Sync + 'static>(name: &str) {
    let handle = FsCollectionHandle::<T>::new(name).expect("Failed to lookup actor handle");
    handle.start().await.expect("Failed to start collection");
}

#[derive(Debug)]
struct FsCollectionConfig {
    pubsub_name: String,
    path: PathBuf,
}

#[derive(Debug)]
struct FsCollection<T: DeserializeOwned + Serialize + Send + Sync + 'static> {
    items: HashMap<String, Item<T>>,
    path: PathBuf,
    refresh_failures: HashSet<String>,
    update: PublisherHandle<Update<T>>,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> Actor for FsCollection<T> {
    type Args = FsCollectionConfig;
    type Error = HandleLookupError;

    async fn on_start(config: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            items: HashMap::new(),
            path: config.path,
            refresh_failures: HashSet::new(),
            update: PublisherHandle::from_name(config.pubsub_name)?,
        })
    }
}

/// Error type for failures that can occur when starting the collection.
#[derive(Error, Debug)]
pub enum StartError {
    #[error("Failed to lookup scheduler handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
    #[error("Failed to initialize scheduler: {0}")]
    SchedulerInit(#[from] CallError),
}

#[derive(Debug)]
struct Start;

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Start>
    for FsCollection<T>
{
    type Reply = Result<(), StartError>;

    async fn handle(
        &mut self,
        _msg: Start,
        ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let scheduler = SchedulerHandle::new()?;

        scheduler
            .set_interval(
                ctx.actor_ref().downgrade(),
                Duration::from_millis(200),
                Refresh,
            )
            .await?;

        Ok(())
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> FsCollection<T> {
    fn make_path(&self, id: &str) -> PathBuf {
        self.path.join(id.to_string() + ".json")
    }
}

#[derive(Debug, Clone)]
struct Refresh;

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Refresh>
    for FsCollection<T>
{
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: Refresh,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut id_set = self.items.keys().cloned().collect::<HashSet<_>>();

        let mut readdir = match fs::read_dir(&self.path).await {
            Ok(rd) => rd,
            Err(e) => {
                tracing::error!(error = ?e, "refresh: failed to read directory");
                return;
            }
        };

        let mut last_refresh_failures = HashSet::new();
        mem::swap(&mut self.refresh_failures, &mut last_refresh_failures);

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
                        // Avoid flooding the logs with repeated errors for the same ID.
                        if !last_refresh_failures.contains(id) {
                            tracing::error!(error = ?e, id, "refresh: error handling item refresh");
                        }

                        self.refresh_failures.insert(id.to_owned());
                        continue;
                    }
                };

                if updated {
                    self.update.publish(Update {
                        id: id.to_owned(),
                        kind: Kind::Updated {
                            value: item.value().clone(),
                        },
                        origin: Origin::External,
                    });
                }

                id_set.remove(id);
            } else {
                let item = match Item::<T>::handle_new(self.make_path(id)).await {
                    Ok(item) => item,
                    Err(e) => {
                        // Avoid flooding the logs with repeated errors for the same ID.
                        if !last_refresh_failures.contains(id) {
                            tracing::error!(error = ?e, id, "refresh: error handling new item");
                        }

                        self.refresh_failures.insert(id.to_owned());
                        continue;
                    }
                };

                let value = item.value().clone();
                self.items.insert(id.to_owned(), item);

                self.update.publish(Update {
                    id: id.to_owned(),
                    kind: Kind::Created { value },
                    origin: Origin::External,
                });
            }
        }

        // Remove items that are no longer present in the filesystem.
        for id in id_set {
            self.items.remove(&id);
            self.update.publish(Update {
                id,
                kind: Kind::Deleted,
                origin: Origin::External,
            });
        }
    }
}

#[derive(Debug)]
struct Set<T> {
    id: String,
    value: Arc<T>,
    kind: SetKind,
}

#[derive(Debug)]
enum SetKind {
    Set,
    Create,
    Update,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Set<T>>
    for FsCollection<T>
{
    type Reply = Result<(), FsCollectionError>;

    async fn handle(
        &mut self,
        msg: Set<T>,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg.kind {
            SetKind::Set => {
                if self.items.contains_key(&msg.id) {
                    self.update(&msg.id, msg.value).await
                } else {
                    self.create(&msg.id, msg.value).await
                }
            }
            SetKind::Create => self.create(&msg.id, msg.value).await,
            SetKind::Update => self.update(&msg.id, msg.value).await,
        }
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> FsCollection<T> {
    async fn create(&mut self, id: &str, value: Arc<T>) -> Result<(), FsCollectionError> {
        if self.items.contains_key(id) {
            return Err(FsCollectionError::AlreadyExists(id.to_owned()));
        }

        let path = self.make_path(id);
        let item = Item::new(path, value).await?;

        let value = item.value().clone();
        self.items.insert(id.to_owned(), item);

        self.update.publish(Update {
            id: id.to_owned(),
            kind: Kind::Created { value },
            origin: Origin::Internal,
        });

        Ok(())
    }

    async fn update(&mut self, id: &str, value: Arc<T>) -> Result<(), FsCollectionError> {
        let item = self
            .items
            .get_mut(id)
            .ok_or_else(|| FsCollectionError::NotFound(id.to_owned()))?;
        item.update(value).await?;

        self.update.publish(Update {
            id: id.to_owned(),
            kind: Kind::Updated {
                value: item.value().clone(),
            },
            origin: Origin::Internal,
        });

        Ok(())
    }
}

#[derive(Debug)]
struct Rename {
    id: String,
    new_id: String,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Rename>
    for FsCollection<T>
{
    type Reply = Result<(), FsCollectionError>;

    async fn handle(
        &mut self,
        msg: Rename,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.items.contains_key(&msg.new_id) {
            return Err(FsCollectionError::AlreadyExists(msg.new_id.to_owned()));
        }
        let new_path = self.make_path(&msg.new_id);

        // do not remove it before real rename
        let item = self
            .items
            .get_mut(&msg.id)
            .ok_or_else(|| FsCollectionError::NotFound(msg.id.to_owned()))?;
        item.rename(new_path).await?;

        let item = self.items.remove(&msg.id).expect("item not found");
        self.items.insert(msg.new_id.to_owned(), item);

        self.update.publish(Update {
            id: msg.id.to_owned(),
            kind: Kind::Renamed {
                new_id: msg.new_id.to_owned(),
            },
            origin: Origin::Internal,
        });

        Ok(())
    }
}

#[derive(Debug)]
struct Delete {
    id: String,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Delete>
    for FsCollection<T>
{
    type Reply = Result<(), FsCollectionError>;

    async fn handle(
        &mut self,
        msg: Delete,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // do not remove it before real delete
        let item = self
            .items
            .get_mut(&msg.id)
            .ok_or_else(|| FsCollectionError::NotFound(msg.id.to_owned()))?;

        item.delete().await?;
        self.items.remove(&msg.id);

        self.update.publish(Update {
            id: msg.id.to_owned(),
            kind: Kind::Deleted,
            origin: Origin::Internal,
        });

        Ok(())
    }
}

#[derive(Debug)]
struct Get {
    id: String,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<Get>
    for FsCollection<T>
{
    type Reply = Result<Arc<T>, FsCollectionError>;

    async fn handle(
        &mut self,
        msg: Get,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let item = self
            .items
            .get(&msg.id)
            .ok_or_else(|| FsCollectionError::NotFound(msg.id.to_owned()))?;

        Ok(item.value().clone())
    }
}

#[derive(Debug)]
struct GetAll;

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> message::Message<GetAll>
    for FsCollection<T>
{
    type Reply = Result<Vec<(String, Arc<T>)>, FsCollectionError>;

    async fn handle(
        &mut self,
        _msg: GetAll,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self
            .items
            .iter()
            .map(|(id, item)| (id.clone(), item.value().clone()))
            .collect())
    }
}

#[derive(Debug)]
struct Item<T: DeserializeOwned + Serialize + Send + Sync + 'static> {
    path: PathBuf,
    modified: SystemTime,
    size: u64,
    raw: Bytes,
    value: Arc<T>,
}

impl<T: DeserializeOwned + Serialize + Send + Sync + 'static> Item<T> {
    /// Creates a new `Item` from the given value, writing it to the specified path.
    pub async fn new(path: PathBuf, value: Arc<T>) -> Result<Self, FsCollectionError> {
        let raw = Bytes::from_owner(serde_json::to_vec(value.as_ref())?);

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
    pub async fn update(&mut self, value: Arc<T>) -> Result<(), FsCollectionError> {
        let raw = Bytes::from_owner(serde_json::to_vec(value.as_ref())?);
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
            value: Arc::new(value),
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
        self.value = Arc::new(value);

        Ok(true)
    }

    /// Returns a reference to the path of the item.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the value contained in the item.
    pub fn value(&self) -> &Arc<T> {
        &self.value
    }
}
