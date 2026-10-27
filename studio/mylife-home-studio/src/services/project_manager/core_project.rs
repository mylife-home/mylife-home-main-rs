use std::{collections::HashMap, mem, sync::Arc, todo};

use common::utils::actors::{CallError, SpawnExt};
use studio_web_api::project_manager;

use crate::{
    services::project_manager::{InitialEmitter, ProjectManagerActorError},
    utils::fs_collection::{self, FsCollectionHandle, Update},
    web::{NotifierManager, SessionEvent, SessionHandle},
};

use kameo::{error::Infallible, message, prelude::*};

pub fn new_core_project_data() -> Arc<project_manager::CoreProject> {
    Arc::new(project_manager::CoreProject {
        components: HashMap::new(),
        plugins: HashMap::new(),
        bindings: HashMap::new(),
        templates: HashMap::new(),
    })
}

#[derive(Debug, Clone)]
pub struct CoreProjectHandle(ActorRef<CoreProject>);

impl CoreProjectHandle {
    pub async fn create(
        name: String,
        fs_collection: FsCollectionHandle<project_manager::CoreProject>,
    ) -> Result<Self, ProjectManagerActorError> {
        let data = fs_collection.get(name.clone()).await?;

        let actor = CoreProject::spawn_unbounded(CoreProjectArgs {
            name,
            data,
            fs_collection,
        });

        // Startup cannot fail
        actor.wait_for_startup().await;

        Ok(Self(actor))
    }

    pub async fn terminate(&self) {
        if let Err(error) = self.0.stop_gracefully().await {
            tracing::error!(%error, "cannot stop project actor");
            return;
        }

        // Shutdown cannot fail
        self.0.wait_for_shutdown().await;
    }

    pub async fn open(
        &self,
        session: SessionHandle,
    ) -> Result<(String, InitialEmitter), ProjectManagerActorError> {
        let res = self.0.ask(Open { session }).try_send().await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn close(
        &self,
        session: SessionHandle,
        notifier_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        let res = self
            .0
            .ask(Close {
                session,
                notifier_id,
            })
            .try_send()
            .await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn call(
        &self,
        session: SessionHandle,
        notifier_id: String,
        call_data: project_manager::ProjectCall,
    ) -> Result<project_manager::ProjectCallResult, ProjectManagerActorError> {
        let res = self
            .0
            .ask(Call {
                call_data,
                session,
                notifier_id,
            })
            .try_send()
            .await;

        Ok(res.map_err(|e| {
            ProjectManagerActorError::OpenedProjectError(Box::new(CallError::from(e)))
        })?)
    }

    pub async fn session_event(&self, event: SessionEvent) -> Vec<String> {
        let res = self.0.ask(event).try_send().await;

        res.unwrap_or_else(|error| {
            tracing::error!(%error, "could not send session event to project actor");
            Vec::new()
        })
    }
}

#[derive(Debug)]
struct CoreProjectArgs {
    name: String,
    data: Arc<project_manager::CoreProject>,
    fs_collection: FsCollectionHandle<project_manager::CoreProject>,
}

/// Represents an opened core project
#[derive(Debug)]
struct CoreProject {
    name: String,
    notifiers: NotifierManager<project_manager::UpdateProjectNotification>,
    data: Arc<project_manager::CoreProject>,
    fs_collection: FsCollectionHandle<project_manager::CoreProject>,
}

impl Actor for CoreProject {
    type Args = CoreProjectArgs;
    type Error = Infallible;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        args.fs_collection.on_update().subscribe(actor_ref);

        Ok(Self {
            name: args.name,
            notifiers: NotifierManager::new("project-manager/opened-project"),
            data: args.data,
            fs_collection: args.fs_collection,
        })
    }
}

#[derive(Debug)]
struct Open {
    session: SessionHandle,
}

impl message::Message<Open> for CoreProject {
    type Reply = Result<(String, InitialEmitter), ProjectManagerActorError>;

    async fn handle(&mut self, msg: Open, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let notifier = self.notifiers.create_notifier(msg.session);

        // TODO: initial notifications
        let notifications = Vec::new();

        let notifier_id = notifier.notifier_id().to_owned();
        let notification_emitter = InitialEmitter::new(notifier.clone(), notifications);

        Ok((notifier_id, notification_emitter))
    }
}

#[derive(Debug)]
struct Close {
    session: SessionHandle,
    notifier_id: String,
}

impl message::Message<Close> for CoreProject {
    type Reply = Result<(), ProjectManagerActorError>;

    async fn handle(&mut self, msg: Close, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let removed = self
            .notifiers
            .remove_notifier(&msg.session, &msg.notifier_id);
        if !removed {
            return Err(ProjectManagerActorError::NotifierNotFound(msg.notifier_id));
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Call {
    session: SessionHandle,
    notifier_id: String,
    call_data: project_manager::ProjectCall,
}

impl message::Message<Call> for CoreProject {
    type Reply = Result<project_manager::ProjectCallResult, ProjectManagerActorError>;

    async fn handle(&mut self, msg: Call, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        todo!()
    }
}

impl message::Message<Update<project_manager::CoreProject>> for CoreProject {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Update<project_manager::CoreProject>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.id != self.name {
            return;
        }

        match msg.kind {
            fs_collection::Kind::Created { value } => {
                tracing::warn!(name = self.name, "unexpected creation of project");
            }
            fs_collection::Kind::Deleted => {
                tracing::warn!(name = self.name, "unexpected deletion of project");
            }

            fs_collection::Kind::Updated { value } => {
                // Was it our update?
                if Arc::ptr_eq(&self.data, &value) {
                    return;
                }

                self.reload(value);
            }

            fs_collection::Kind::Renamed { new_id } => {
                let mut old_name = new_id;
                mem::swap(&mut self.name, &mut old_name);

                tracing::debug!(old_name, new_name = self.name, "project renamed");

                self.notifiers
                    .notify_all(&project_manager::UpdateProjectNotification::SetName(
                        project_manager::SetNameProjectNotification {
                            name: self.name.clone(),
                        },
                    ));
            }
        }
    }
}

impl CoreProject {
    fn reload(&mut self, value: Arc<project_manager::CoreProject>) {
        // TODO
        self.data = value;
    }
}

impl message::Message<SessionEvent> for CoreProject {
    type Reply = Vec<String>;

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg)
    }
}
