//! Git over the studio's data root.
//!
//! The service watches one repository: the directory the projects are stored
//! in. It fetches on a timer, recomputes status when files move, and offers
//! the designer commit, restore and diff.
//!
//! A changed file is reported under a *feature*, the name a service gives to
//! the paths it owns. Features are collected by [`GitBuilder`] while the other
//! services start, in the same way the dispatcher collects its handlers, so
//! the actor is built once knowing all of them.

mod repo;

use std::{collections::BTreeSet, path::PathBuf, time::Duration};

use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};
use kameo::{message, prelude::*};
use studio_web_api::{git as wire, protocol};

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

pub use repo::{Credentials, SshKey, UserPass};

const GIT_NAME: &str = "git";

/// How often the repository is fetched, to keep ahead/behind honest.
const FETCH_INTERVAL: Duration = Duration::from_secs(6);

/// How long a burst of file writes is let settle before one status pass.
const STATUS_DEBOUNCE: Duration = Duration::from_millis(100);

/// The paths one service owns, under the name the designer sees.
#[derive(Debug, Clone)]
struct PathFeature {
    name: String,
    path: PathBuf,
}

/// Collects the path features before the actor is built.
#[derive(Debug)]
pub struct GitBuilder {
    root: PathBuf,
    app_url: Option<String>,
    credentials: Credentials,
    features: Vec<PathFeature>,
}

impl GitBuilder {
    pub fn new(root: PathBuf, app_url: Option<String>) -> Self {
        Self {
            root,
            app_url,
            credentials: Credentials::default(),
            features: Vec::new(),
        }
    }

    /// What the remotes are reached with.
    pub fn credentials(&mut self, credentials: Credentials) {
        self.credentials = credentials;
    }

    /// Names the paths a feature owns. A changed file is reported under the
    /// first feature whose path contains it, so overlapping paths resolve in
    /// registration order.
    pub fn register_path_feature(
        &mut self,
        name: impl Into<String>,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        let name = name.into();

        for path in paths {
            let path = self.root.join(path);
            tracing::debug!(feature = %name, path = %path.display(), "registering path feature");
            self.features.push(PathFeature {
                name: name.clone(),
                path,
            });
        }
    }
}

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    builder: GitBuilder,
) {
    let (git, _) = SpawnedActor::start::<Git>(builder).await;

    git.register(GIT_NAME);

    actors.add(git);

    let actor: ActorRef<_> = ActorHandle::<Git>::from_name(GIT_NAME)
        .expect("cannot get git actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("git/start-notify", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("git/stop-notify", actor.clone());
    dispatcher.register_call::<RefreshReq, _>("git/refresh", actor.clone());
    dispatcher.register_call::<CommitReq, _>("git/commit", actor.clone());
    dispatcher.register_call::<RestoreReq, _>("git/restore", actor.clone());
    dispatcher.register_call::<DiffReq, _>("git/diff", actor);
}

/// Reaches the running git service, for the other services to say a file moved.
#[derive(Debug, Clone)]
pub struct GitHandle(ActorRef<Git>);

impl GitHandle {
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::<Git>::from_name(GIT_NAME)?.into()))
    }

    /// Says the working tree moved. Status is recomputed shortly after, once
    /// the writes around this one have settled.
    pub fn notify_file_update(&self) {
        if let Err(error) = self.0.tell(NotifyFileUpdate).try_send() {
            tracing::error!(%error, "could not notify git of a file update");
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct StartNotifyReq;

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct StartNotifyRes(protocol::NotifierId);

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct StopNotifyReq(protocol::NotifierId);

#[derive(Debug, serde::Serialize)]
struct StopNotifyRes;

/// Braces, not a unit struct: the client sends `{}` for a call with no
/// arguments, and a unit struct only deserializes from `null`.
#[derive(Debug, serde::Deserialize)]
struct RefreshReq {}

#[derive(Debug, serde::Serialize)]
struct RefreshRes;

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct CommitReq(wire::GitCommit);

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct RestoreReq(wire::GitRestore);

#[derive(Debug, serde::Deserialize)]
struct DiffReq {}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
struct DiffRes(wire::GitDiff);

/// The fetch timer fired.
#[derive(Debug)]
struct Tick;

/// A background fetch ended, with what it came back with.
#[derive(Debug)]
struct Fetched(Result<(), repo::RepoError>);

/// The debounce elapsed: read the repository and tell the designers.
#[derive(Debug)]
struct ComputeStatus;

/// A service wrote to the working tree.
#[derive(Debug)]
struct NotifyFileUpdate;

#[derive(Debug)]
struct Git {
    repo: repo::Repository,
    features: Vec<PathFeature>,
    notifiers: NotifierManager<wire::GitStatusNotification>,
    status: wire::GitStatus,
    /// Set while a status pass is already waiting out the debounce.
    status_scheduled: bool,
    /// Set while a fetch is running in the background.
    fetch_scheduled: bool,
}

impl Actor for Git {
    type Args = GitBuilder;
    type Error = HandleLookupError;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // The first status pass owes nothing to the network: a repository
        // with an unreachable remote still has a branch and local changes to
        // show.
        let _ = actor_ref.tell(ComputeStatus).try_send();

        let fetcher = actor_ref.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(FETCH_INTERVAL);

            loop {
                interval.tick().await;

                if fetcher.tell(Tick).try_send().is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            repo: repo::Repository::new(args.root, args.credentials),
            features: args.features,
            notifiers: NotifierManager::new("git/status"),
            status: wire::GitStatus {
                app_url: args.app_url,
                branch: String::from("<unknown>"),
                changed_features: Vec::new(),
                ahead: None,
                behind: None,
            },
            status_scheduled: false,
            fetch_scheduled: false,
        })
    }
}

impl Git {
    /// Recomputes status and tells the designers, but only about what moved.
    async fn update_status(&mut self) {
        let status = match self.repo.status().await {
            Ok(status) => status,
            Err(error) => {
                tracing::error!(%error, "could not read the repository status");
                return;
            }
        };

        let changed_features: Vec<String> = status
            .changed_files
            .iter()
            .filter_map(|file| self.find_feature(file))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let updated = wire::GitStatus {
            app_url: self.status.app_url.clone(),
            branch: status.branch,
            changed_features,
            ahead: status.ahead,
            behind: status.behind,
        };

        if updated == self.status {
            return;
        }

        self.status = updated;

        tracing::debug!(
            branch = %self.status.branch,
            changed_features = ?self.status.changed_features,
            ahead = ?self.status.ahead,
            behind = ?self.status.behind,
            "status changed"
        );

        self.notifiers.notify_all(&wire::GitStatusNotification {
            status: self.status.clone(),
        });
    }

    /// Fetches off the mailbox: the actor has calls to serve while the
    /// network takes its time, and a fetch slower than the interval must not
    /// let the ticks pile up behind it.
    fn schedule_fetch(&mut self, actor_ref: ActorRef<Self>) {
        if self.fetch_scheduled {
            return;
        }

        self.fetch_scheduled = true;

        let repo = self.repo.clone();

        tokio::spawn(async move {
            let result = repo.fetch().await;
            let _ = actor_ref.tell(Fetched(result)).try_send();
        });
    }

    /// Waits out a burst of writes before reading the repository.
    fn schedule_status(&mut self, actor_ref: ActorRef<Self>) {
        if self.status_scheduled {
            return;
        }

        self.status_scheduled = true;

        tokio::spawn(async move {
            tokio::time::sleep(STATUS_DEBOUNCE).await;
            let _ = actor_ref.tell(ComputeStatus).try_send();
        });
    }

    /// The feature a file belongs to, by the first registered path containing it.
    fn find_feature(&self, file: &str) -> Option<String> {
        let path = self.repo.root().join(file);

        self.features
            .iter()
            .find(|feature| path.starts_with(&feature.path))
            .map(|feature| feature.name.clone())
    }

    async fn diff(&self) -> Result<wire::GitDiff, repo::RepoError> {
        let mut files = self.repo.diff().await?;

        for file in &mut files {
            if let Some(path) = file.to.as_ref().or(file.from.as_ref()) {
                file.feature = self.find_feature(path);
            }
        }

        Ok(wire::GitDiff { files })
    }
}

impl message::Message<SessionEvent> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<NotifyFileUpdate> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: NotifyFileUpdate,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.schedule_status(ctx.actor_ref().clone());
    }
}

impl message::Message<ComputeStatus> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ComputeStatus,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.status_scheduled = false;
        self.update_status().await;
    }
}

impl message::Message<Tick> for Git {
    type Reply = ();

    async fn handle(&mut self, _msg: Tick, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.schedule_fetch(ctx.actor_ref().clone());
    }
}

impl message::Message<Fetched> for Git {
    type Reply = ();

    async fn handle(&mut self, msg: Fetched, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.fetch_scheduled = false;

        if let Err(error) = msg.0 {
            tracing::error!(%error, "could not fetch");
        }

        // Even a failed fetch leaves the local side worth reading again.
        self.schedule_status(ctx.actor_ref().clone());
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StartNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier = self.notifiers.create_notifier(call.session().clone());

        call.reply_ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        }));

        notifier.notify(&wire::GitStatusNotification {
            status: self.status.clone(),
        });
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<StopNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let notifier_id = &call.request().0;
        self.notifiers
            .remove_notifier(call.session(), notifier_id.notifier_id.as_str());

        call.reply_ok(StopNotifyRes);
    }
}

impl message::Message<ServiceRequest<RefreshReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<RefreshReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();

        // Behind means take what is there, ahead means send what is here, and
        // neither means look again. The actor serves one call at a time, so
        // two refreshes cannot overlap. This one does hold the mailbox for as
        // long as the network takes, which is what the designer's spinner is
        // waiting on anyway.
        let result = if self.status.behind.unwrap_or(0) > 0 {
            self.repo.pull().await
        } else if self.status.ahead.unwrap_or(0) > 0 {
            self.repo.push().await
        } else {
            self.repo.fetch().await
        };

        match result {
            Ok(()) => {
                self.schedule_status(ctx.actor_ref().clone());
                call.reply_ok(RefreshRes);
            }
            Err(error) => call.reply_error(error),
        }
    }
}

impl message::Message<ServiceRequest<CommitReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<CommitReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let CommitReq(data) = call.request();

        let result = self
            .repo
            .commit(data.message.clone(), data.files.clone())
            .await;

        if let Err(error) = result {
            call.reply_error(error);
            return;
        }

        self.schedule_status(ctx.actor_ref().clone());

        match self.diff().await {
            Ok(diff) => call.reply_ok(DiffRes(diff)),
            Err(error) => call.reply_error(error),
        }
    }
}

impl message::Message<ServiceRequest<RestoreReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<RestoreReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let RestoreReq(data) = call.request();

        if let Err(error) = self.repo.restore(data.files.clone()).await {
            call.reply_error(error);
            return;
        }

        self.schedule_status(ctx.actor_ref().clone());

        match self.diff().await {
            Ok(diff) => call.reply_ok(DiffRes(diff)),
            Err(error) => call.reply_error(error),
        }
    }
}

impl message::Message<ServiceRequest<DiffReq>> for Git {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<DiffReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();

        match self.diff().await {
            Ok(diff) => call.reply_ok(DiffRes(diff)),
            Err(error) => call.reply_error(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `start-notify` is called with `null`, and the calls that take no
    /// arguments are called with `{}`. Both have to land.
    #[test]
    fn the_payloads_the_client_sends_deserialize() {
        serde_json::from_value::<StartNotifyReq>(serde_json::json!(null)).unwrap();
        serde_json::from_value::<RefreshReq>(serde_json::json!({})).unwrap();
        serde_json::from_value::<DiffReq>(serde_json::json!({})).unwrap();
        serde_json::from_value::<StopNotifyReq>(serde_json::json!({ "notifierId": "n" })).unwrap();
        serde_json::from_value::<CommitReq>(serde_json::json!({ "message": "m", "files": [] }))
            .unwrap();
        serde_json::from_value::<RestoreReq>(serde_json::json!({ "files": [] })).unwrap();
    }
}
