use std::{collections::HashMap, sync::Arc, time::Duration};

use common::{
    InitData,
    bus::{
        metadata::{MetadataHandle, RemoteUpdate},
        rpc::{RpcClientError, RpcHandle},
    },
    instance_info::{
        InstanceInfoProviderHandle,
        types::{self, InstanceInfo},
    },
    utils::actors::{ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{message, prelude::*};
use studio_web_api::{core_model, online, protocol, ui_model};
use thiserror::Error;

use crate::web::{DispatcherBuilder, NotifierManager, ServiceRequest, SessionEvent};

const ONLINE_INSTANCES_NAME: &str = "online-instances";
const INSTANCE_INFO_PATH: &str = "instance-info";

/// Client access to the client actor
#[derive(Debug, Clone)]
pub struct OnlineInstancesHandle(ActorHandle<OnlineInstances>);

impl OnlineInstancesHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::<OnlineInstances>::from_name(
            ONLINE_INSTANCES_NAME,
        )?))
    }

    /// The online instances declaring a capability, such as `bindings-api`.
    ///
    /// A deploy needs to know there is exactly one of some of these, and
    /// validation reports when there is not.
    pub async fn instances_with_capability(
        &self,
        capability: String,
    ) -> Result<Vec<String>, CallError<OnlineInstancesError>> {
        self.0.call(InstancesWithCapability(capability)).await
    }

    /// What an instance currently runs, and what to change about it.
    ///
    /// These are the four calls a core deploy makes, plus the save that keeps
    /// the result across a restart.
    pub async fn core_list_components(
        &self,
        instance_name: String,
    ) -> Result<Vec<core_model::ComponentConfig>, CallError<OnlineInstancesError>> {
        self.0.call(CoreListComponents(instance_name)).await
    }

    pub async fn core_add_component(
        &self,
        instance_name: String,
        config: core_model::ComponentConfig,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(CoreAddComponent(instance_name, config)).await
    }

    pub async fn core_remove_component(
        &self,
        instance_name: String,
        id: String,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(CoreRemoveComponent(instance_name, id)).await
    }

    pub async fn core_list_bindings(
        &self,
        instance_name: String,
    ) -> Result<Vec<core_model::BindingConfig>, CallError<OnlineInstancesError>> {
        self.0.call(CoreListBindings(instance_name)).await
    }

    pub async fn core_add_binding(
        &self,
        instance_name: String,
        config: core_model::BindingConfig,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(CoreAddBinding(instance_name, config)).await
    }

    pub async fn core_remove_binding(
        &self,
        instance_name: String,
        config: core_model::BindingConfig,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(CoreRemoveBinding(instance_name, config)).await
    }

    pub async fn core_store_save(
        &self,
        instance_name: String,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(CoreStoreSave(instance_name)).await
    }

    /// Deploy a UI definition to the online instances
    pub async fn deploy_ui_definition(
        &self,
        definition: ui_model::Definition,
    ) -> Result<(), CallError<OnlineInstancesError>> {
        self.0.call(DeployUiDefinition(definition)).await
    }
}

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    init_data: &InitData,
) {
    let (online_instances, _) =
        SpawnedActor::start::<OnlineInstances>(init_data.instance_name.clone()).await;

    online_instances.register(ONLINE_INSTANCES_NAME);
    actors.add(online_instances);

    let actor: ActorRef<_> = ActorHandle::<OnlineInstances>::from_name(ONLINE_INSTANCES_NAME)
        .expect("cannot get online instances actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher
        .register_call::<StartNotifyReq, _>("online/start-notify-instance-info", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("online/stop-notify-instance-info", actor.clone());
    dispatcher.register_call::<ExecuteSystemRestartReq, _>(
        "online/execute-system-restart",
        actor.clone(),
    );
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

#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct ExecuteSystemRestartReq(online::SystemRestart);

#[derive(Debug, serde::Serialize)]
struct ExecuteSystemRestartRes;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemRestartRpcReq {
    fail_safe: bool,
}

/// The response is an empty object, not a null value
#[derive(Debug, serde::Deserialize)]
struct SystemRestartRpcRes {}

#[derive(Debug, Error)]
pub enum OnlineInstancesError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
    #[error("rpc call failed: {0}")]
    Rpc(#[from] RpcClientError),
    #[error("instance '{instance_name}' does not have capability '{capability}'")]
    MissingCapability {
        instance_name: String,
        capability: &'static str,
    },
    #[error("no online instance has capability 'ui-api'")]
    UiInstanceNotFound,
    #[error("multiple online instances have capability 'ui-api'")]
    MultipleUiInstances,
}

#[derive(Debug)]
struct OnlineInstances {
    local_instance_name: Arc<String>,
    rpc: RpcHandle,
    instances: HashMap<String, online::InstanceInfo>,
    notifiers: NotifierManager<online::UpdateInstanceInfoData>,
}

impl Actor for OnlineInstances {
    type Args = Arc<String>;
    type Error = OnlineInstancesError;

    async fn on_start(
        instance_name: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;
        let instance_info = InstanceInfoProviderHandle::new()?;
        let rpc = RpcHandle::new()?;

        metadata.on_remote_update().subscribe(actor_ref.clone());
        instance_info.on_event().subscribe(actor_ref.clone());

        Ok(Self {
            local_instance_name: instance_name,
            rpc,
            instances: HashMap::new(),
            notifiers: NotifierManager::new("online/instance-info"),
        })
    }
}

impl message::Message<SessionEvent> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<RemoteUpdate> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        update: RemoteUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if update.path() != INSTANCE_INFO_PATH {
            return;
        }

        let instance_name = update.instance().to_owned();
        if update.has_value() {
            match update.read_value::<InstanceInfo>() {
                Ok(info) => self.set_instance(instance_name, convert_instance_info(info)),
                Err(error) => {
                    tracing::error!(%error, instance = update.instance(), "could not read remote instance info")
                }
            }
        } else {
            self.clear_instance(&instance_name);
        }
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for OnlineInstances {
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

        for (instance_name, data) in &self.instances {
            notifier.notify(&online::UpdateInstanceInfoData::Set(
                online::SetInstanceInfoData {
                    instance_name: instance_name.clone(),
                    data: data.clone(),
                },
            ));
        }
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for OnlineInstances {
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

impl message::Message<ServiceRequest<ExecuteSystemRestartReq>> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        request: ServiceRequest<ExecuteSystemRestartReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let call = request.into_call();
        let request = call.request().0.clone();
        let result = self.execute_system_restart(request).await;
        call.reply_result(result);
    }
}

impl OnlineInstances {
    async fn execute_system_restart(
        &mut self,
        request: online::SystemRestart,
    ) -> Result<ExecuteSystemRestartRes, OnlineInstancesError> {
        let Some(instance) = self.instances.get(&request.instance_name) else {
            return Err(OnlineInstancesError::MissingCapability {
                instance_name: request.instance_name,
                capability: "restart-api",
            });
        };

        if !instance
            .capabilities
            .iter()
            .any(|capability| capability == "restart-api")
        {
            return Err(OnlineInstancesError::MissingCapability {
                instance_name: request.instance_name,
                capability: "restart-api",
            });
        }

        self.rpc
            .call::<_, SystemRestartRpcRes>(
                request.instance_name,
                "system.restart",
                &SystemRestartRpcReq {
                    fail_safe: request.fail_safe,
                },
                None,
            )
            .await?;

        Ok(ExecuteSystemRestartRes)
    }
}

impl message::Message<types::InstanceInfo> for OnlineInstances {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: types::InstanceInfo,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_instance(
            self.local_instance_name.as_ref().clone(),
            convert_instance_info(msg),
        );
    }
}

const COMPONENTS_API: &str = "components-api";
const BINDINGS_API: &str = "bindings-api";
const STORE_API: &str = "store-api";

/// A store save walks the disk on the target, which on a small board is slow
/// enough that the default timeout is not enough.
const STORE_SAVE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct CoreListComponents(String);

impl message::Message<CoreListComponents> for OnlineInstances {
    type Reply = Result<Vec<core_model::ComponentConfig>, OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreListComponents,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, COMPONENTS_API)?;
        Ok(self
            .rpc
            .call::<_, Vec<core_model::ComponentConfig>>(msg.0, "components.list", &(), None)
            .await?)
    }
}

#[derive(Debug)]
struct CoreAddComponent(String, core_model::ComponentConfig);

impl message::Message<CoreAddComponent> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreAddComponent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, COMPONENTS_API)?;
        self.rpc
            .call::<_, ()>(msg.0, "components.add", &msg.1, None)
            .await?;
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
struct ComponentRemoveRpcReq {
    id: String,
}

#[derive(Debug)]
struct CoreRemoveComponent(String, String);

impl message::Message<CoreRemoveComponent> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreRemoveComponent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, COMPONENTS_API)?;
        self.rpc
            .call::<_, ()>(
                msg.0,
                "components.remove",
                &ComponentRemoveRpcReq { id: msg.1 },
                None,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
struct CoreListBindings(String);

impl message::Message<CoreListBindings> for OnlineInstances {
    type Reply = Result<Vec<core_model::BindingConfig>, OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreListBindings,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, BINDINGS_API)?;
        Ok(self
            .rpc
            .call::<_, Vec<core_model::BindingConfig>>(msg.0, "bindings.list", &(), None)
            .await?)
    }
}

#[derive(Debug)]
struct CoreAddBinding(String, core_model::BindingConfig);

impl message::Message<CoreAddBinding> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreAddBinding,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, BINDINGS_API)?;
        self.rpc
            .call::<_, ()>(msg.0, "bindings.add", &msg.1, None)
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
struct CoreRemoveBinding(String, core_model::BindingConfig);

impl message::Message<CoreRemoveBinding> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreRemoveBinding,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, BINDINGS_API)?;
        self.rpc
            .call::<_, ()>(msg.0, "bindings.remove", &msg.1, None)
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
struct CoreStoreSave(String);

impl message::Message<CoreStoreSave> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: CoreStoreSave,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.check_capability(&msg.0, STORE_API)?;
        self.rpc
            .call::<_, ()>(msg.0, "store.save", &(), Some(STORE_SAVE_TIMEOUT))
            .await?;
        Ok(())
    }
}

#[derive(Debug)]
struct InstancesWithCapability(String);

impl message::Message<InstancesWithCapability> for OnlineInstances {
    type Reply = Result<Vec<String>, OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: InstancesWithCapability,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.with_capability(&msg.0).collect())
    }
}

#[derive(Debug)]
struct DeployUiDefinition(ui_model::Definition);

impl message::Message<DeployUiDefinition> for OnlineInstances {
    type Reply = Result<(), OnlineInstancesError>;

    async fn handle(
        &mut self,
        msg: DeployUiDefinition,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut ui_instances = self.with_capability("ui-api");
        let Some(instance_name) = ui_instances.next() else {
            return Err(OnlineInstancesError::UiInstanceNotFound);
        };
        if ui_instances.next().is_some() {
            return Err(OnlineInstancesError::MultipleUiInstances);
        }

        self.rpc
            .call::<_, ()>(instance_name, "definition.set", &msg.0, None)
            .await?;

        Ok(())
    }
}

impl OnlineInstances {
    fn check_capability(
        &self,
        instance_name: &str,
        capability: &'static str,
    ) -> Result<(), OnlineInstancesError> {
        let declared = self
            .instances
            .get(instance_name)
            .is_some_and(|info| info.capabilities.iter().any(|item| item == capability));

        if declared {
            return Ok(());
        }

        Err(OnlineInstancesError::MissingCapability {
            instance_name: instance_name.to_owned(),
            capability,
        })
    }

    fn with_capability<'a>(&'a self, capability: &'a str) -> impl Iterator<Item = String> + 'a {
        self.instances
            .iter()
            .filter(move |(_, info)| {
                info.capabilities
                    .iter()
                    .any(|declared| declared == capability)
            })
            .map(|(name, _)| name.clone())
    }

    fn set_instance(&mut self, instance_name: String, data: online::InstanceInfo) {
        self.instances.insert(instance_name.clone(), data.clone());

        self.notifiers
            .notify_all(&online::UpdateInstanceInfoData::Set(
                online::SetInstanceInfoData {
                    instance_name,
                    data,
                },
            ));
    }

    fn clear_instance(&mut self, instance_name: &str) {
        if self.instances.remove(instance_name).is_none() {
            return;
        }

        self.notifiers
            .notify_all(&online::UpdateInstanceInfoData::Clear(
                online::ClearInstanceInfoData {
                    instance_name: instance_name.to_owned(),
                },
            ));
    }
}

fn convert_instance_info(info: InstanceInfo) -> online::InstanceInfo {
    online::InstanceInfo {
        r#type: info.r#type,
        hardware: info.hardware,
        versions: info.versions,
        system_uptime: info.system_uptime.as_secs(),
        instance_uptime: info.instance_uptime.as_secs(),
        hostname: info.hostname,
        capabilities: info.capabilities,
        wifi: info.wifi.map(|wifi| online::InstanceInfoWifi {
            rssi: wifi.rssi as i32,
        }),
    }
}
