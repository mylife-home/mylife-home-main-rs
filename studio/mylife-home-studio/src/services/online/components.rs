use std::collections::HashSet;

use common::{
    components::{
        metadata::{self, ConfigType, MemberType, PluginMetadata}, registry::{RegistryHandle, RegistryUpdated}, types::Value,
    }, utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{error::Infallible, message, prelude::*};
use studio_web_api::{component_model, online, protocol};
use thiserror::Error;

use crate::web::{DispatcherBuilder, Notifier, NotifierManager, ServiceRequest, SessionEvent};

const ONLINE_COMPONENTS_NAME: &str = "online-components";

pub async fn init(actors: &mut SpawnedActors, dispatcher: &mut DispatcherBuilder) {
    let (online_components, _) = SpawnedActor::start::<OnlineComponents>(()).await;

    online_components.register(ONLINE_COMPONENTS_NAME);
    actors.add(online_components);

    let actor: ActorRef<_> = ActorHandle::<OnlineComponents>::from_name(ONLINE_COMPONENTS_NAME)
        .expect("cannot get online components actor handle")
        .into();

    dispatcher.register_session_handler(actor.clone());
    dispatcher.register_call::<StartNotifyReq, _>("online/start-notify-component", actor.clone());
    dispatcher.register_call::<StopNotifyReq, _>("online/stop-notify-component", actor);
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

#[derive(Debug, Error)]
enum OnlineComponentsError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookup(#[from] HandleLookupError),
}

#[derive(Debug)]
struct OnlineComponents {
    registry: RegistryHandle,
    notifiers: NotifierManager<online::UpdateComponentData>,
}

impl Actor for OnlineComponents {
    type Args = ();
    type Error = OnlineComponentsError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let registry = RegistryHandle::new()?;
        registry.on_update().subscribe(actor_ref);

        Ok(Self {
            registry,
            notifiers: NotifierManager::new("online/component"),
        })
    }
}

impl message::Message<SessionEvent> for OnlineComponents {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.notifiers.session_event(&msg);
    }
}

impl message::Message<RegistryUpdated> for OnlineComponents {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RegistryUpdated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.handle_update(msg);
    }
}

impl message::Message<ServiceRequest<StartNotifyReq>> for OnlineComponents {
    type Reply = DelegatedReply<Result<StartNotifyRes, Infallible>>;

    async fn handle(
        &mut self,
        msg: ServiceRequest<StartNotifyReq>,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (session, _) = msg.into();
        let notifier = self.notifiers.create_notifier(session).clone();
        let response = ctx.reply(Ok(StartNotifyRes(protocol::NotifierId {
            notifier_id: notifier.notifier_id().into(),
        })));

        self.initial_sync(&notifier).await;

        response
    }
}

impl message::Message<ServiceRequest<StopNotifyReq>> for OnlineComponents {
    type Reply = Result<StopNotifyRes, Infallible>;

    async fn handle(
        &mut self,
        msg: ServiceRequest<StopNotifyReq>,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (_, notifier_id) = msg.into();
        self.notifiers.remove_notifier(&notifier_id.0.notifier_id);
        Ok(StopNotifyRes)
    }
}

impl OnlineComponents {
    async fn initial_sync(&self, notifier: &Notifier<online::UpdateComponentData>) {
        let components = match self.registry.get_components().await {
            Ok(components) => components,
            Err(error) => {
                tracing::error!(%error, "could not query component registry for initial sync");
                Vec::new()
            }
        };

        let mut plugins = HashSet::new();
        for info in &components {
            let instance_name = Self::instance_name(info.instance.as_deref());
            if plugins.insert((instance_name.clone(), info.plugin.id().to_owned())) {
                notifier.notify(&online::UpdateComponentData::Set(
                    online::SetData::Plugin(online::SetPluginData {
                        instance_name,
                        data: Self::convert_plugin(&info.plugin),
                    }),
                ));
            }
        }

        for info in &components {
            notifier.notify(&online::UpdateComponentData::Set(
                online::SetData::Component(online::SetComponentData {
                    instance_name: Self::instance_name(info.instance.as_deref()),
                    data: component_model::Component {
                        id: info.component_id.clone(),
                        plugin: info.plugin.id().to_owned(),
                    },
                }),
            ));

            for (name, value) in &info.state {
                notifier.notify(&online::UpdateComponentData::Set(
                    online::SetData::State(online::SetStateData {
                        instance_name: Self::instance_name(info.instance.as_deref()),
                        data: online::State {
                            component: info.component_id.clone(),
                            name: name.clone(),
                            value: value
                                .as_ref()
                                .map(Self::convert_value)
                                .unwrap_or(serde_json::Value::Null),
                        },
                    }),
                ));
            }
        }
    }

    fn instance_name(instance: Option<&str>) -> String {
        instance.unwrap_or("local").to_owned()
    }

    fn convert_plugin(plugin: &PluginMetadata) -> component_model::Plugin {
        component_model::Plugin {
            name: plugin.name().to_owned(),
            module: plugin.module().to_owned(),
            usage: Self::convert_usage(plugin.usage()),
            version: plugin.version().to_owned(),
            description: plugin.description().unwrap_or_default().to_owned(),
            members: plugin
                .members()
                .iter()
                .map(|(name, member)| (name.clone(), Self::convert_member(member)))
                .collect(),
            config: plugin
                .config()
                .iter()
                .map(|(name, item)| (name.clone(), Self::convert_config_item(item)))
                .collect(),
        }
    }

    fn convert_member(member: &metadata::Member) -> component_model::Member {
        component_model::Member {
            description: member.description().unwrap_or_default().to_owned(),
            member_type: match member.member_type() {
                MemberType::Action => component_model::MemberType::Action,
                MemberType::State => component_model::MemberType::State,
            },
            value_type: member.value_type().to_string(),
        }
    }

    fn convert_config_item(item: &metadata::ConfigItem) -> component_model::ConfigItem {
        component_model::ConfigItem {
            description: item.description().unwrap_or_default().to_owned(),
            value_type: match item.value_type() {
                ConfigType::String => component_model::ConfigType::String,
                ConfigType::Bool => component_model::ConfigType::Bool,
                ConfigType::Integer => component_model::ConfigType::Integer,
                ConfigType::Float => component_model::ConfigType::Float,
            },
        }
    }

    fn convert_usage(usage: metadata::PluginUsage) -> component_model::PluginUsage {
        match usage {
            metadata::PluginUsage::Sensor => component_model::PluginUsage::Sensor,
            metadata::PluginUsage::Actuator => {
                component_model::PluginUsage::Actuator
            }
            metadata::PluginUsage::Logic => component_model::PluginUsage::Logic,
            metadata::PluginUsage::Ui => component_model::PluginUsage::Ui,
        }
    }

    fn convert_value(value: &Value) -> serde_json::Value {
        match value {
            Value::Range(value) => (*value).into(),
            Value::Text(value) | Value::Enum(value) => value.clone().into(),
            Value::Float(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Bool(value) => (*value).into(),
            Value::Complex => serde_json::Value::Null,
        }
    }

    fn handle_update(&mut self, update: RegistryUpdated) {
        let update = match update {
            RegistryUpdated::PluginAdded(data) => {
                online::UpdateComponentData::Set(online::SetData::Plugin(online::SetPluginData {
                    instance_name: Self::instance_name(data.instance()),
                    data: Self::convert_plugin(data.plugin()),
                }))
            }
            RegistryUpdated::PluginRemoved(data) => {
                online::UpdateComponentData::Clear(online::ClearData {
                    instance_name: Self::instance_name(data.instance()),
                    r#type: online::ComponentDataType::Plugin,
                    id: data.plugin().id().to_owned(),
                })
            }
            RegistryUpdated::ComponentAdded(data) => online::UpdateComponentData::Set(
                online::SetData::Component(online::SetComponentData {
                    instance_name: Self::instance_name(data.instance()),
                    data: component_model::Component {
                        id: data.component_id().to_owned(),
                        plugin: data.plugin().id().to_owned(),
                    },
                }),
            ),
            RegistryUpdated::ComponentRemoved(data) => {
                online::UpdateComponentData::Clear(online::ClearData {
                    instance_name: Self::instance_name(data.instance()),
                    r#type: online::ComponentDataType::Component,
                    id: data.component_id().to_owned(),
                })
            }
            RegistryUpdated::ComponentStateChanged(data) => {
                online::UpdateComponentData::Set(online::SetData::State(online::SetStateData {
                    instance_name: Self::instance_name(data.instance()),
                    data: online::State {
                        component: data.component_id().to_owned(),
                        name: data.state().to_owned(),
                        value: Self::convert_value(data.value()),
                    },
                }))
            }
        };

        self.notifiers.notify_all(&update);
    }
}

