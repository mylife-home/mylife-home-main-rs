use std::{
    collections::HashMap,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bytes::Bytes;
use kameo::{Actor, message, prelude::*};
use rand::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;

use crate::{
    bus::client::{self, ClientHandle, Topic, TopicBuilder},
    utils::actors::{
        ActorHandle, CallError, HandleLookupError, SchedulerHandle, SpawnedActor, SpawnedActors,
    },
};

const DOMAIN: &str = "rpc";
const SERVICES: &str = "services";
const REPLIES: &str = "replies";

const RPC_NAME: &str = "bus.rpc";

const TIMEOUT_CHECK_INTERVAL: Duration = Duration::from_millis(1000);
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);

#[derive(Debug)]
pub struct RpcConfig {
    pub instance_name: Arc<String>,
}

/// Error that occurs during RPC client operations
#[derive(Debug, Error)]
pub enum RpcClientError {
    #[error("cannot serialize request: {0}")]
    Serialization(#[source] serde_json::Error),
    #[error("cannot deserialize reply: {0}")]
    Deserialization(#[source] serde_json::Error),
    #[error("call error: {0}")]
    CallError(#[from] CallError<RpcCallError>),
}

/// Client access to the RPC actor
#[derive(Debug, Clone)]
pub struct RpcHandle(ActorHandle<Rpc>);

impl RpcHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(RPC_NAME)?))
    }

    /// Register a new RPC service
    pub async fn register_service<Impl, Request, Reply>(
        &self,
        address: impl Into<String>,
        implementation: Impl,
    ) -> Result<(), CallError<RpcServiceAddError>>
    where
        Impl: RpcService<Request = Request, Reply = Reply> + 'static,
        Request: DeserializeOwned + 'static,
        Reply: Serialize + 'static,
    {
        self.0
            .call(ServiceAdd {
                address: address.into(),
                service: Box::new(TypedServiceAddImpl::new(implementation)),
            })
            .await?;

        Ok(())
    }

    /// Unregister an RPC service
    pub async fn unregister_service(
        &self,
        address: impl Into<String>,
    ) -> Result<(), CallError<RpcServiceRemoveError>> {
        self.0
            .call(ServiceRemove {
                address: address.into(),
            })
            .await?;

        Ok(())
    }

    pub async fn call<Request, Reply>(
        &self,
        target_instance: impl Into<String>,
        address: impl Into<String>,
        data: &Request,
        timeout: Option<Duration>,
    ) -> Result<Reply, RpcClientError>
    where
        Request: Serialize + 'static,
        Reply: DeserializeOwned + 'static,
    {
        let input = serde_json::to_value(&data).map_err(|e| RpcClientError::Serialization(e))?;

        let output = self
            .0
            .call(Call {
                target_instance: target_instance.into(),
                address: address.into(),
                input,
                timeout,
            })
            .await?;

        Ok(serde_json::from_value(output).map_err(|e| RpcClientError::Deserialization(e))?)
    }
}

pub async fn init_actor(actors: &mut SpawnedActors, config: RpcConfig) {
    let (rpc, _) = SpawnedActor::start::<Rpc>(config).await;

    rpc.register(RPC_NAME);

    actors.add(rpc);
}

struct Rpc {
    client: ClientHandle,
    instance_name: Arc<String>,
    services: HashMap<String, Arc<dyn ServiceHandler>>,
    client_calls: Vec<ClientCall>,
}

impl Rpc {
    fn clean_client_calls(&mut self) {
        self.client_calls.retain(|call| !call.is_terminated());
    }
}

impl Actor for Rpc {
    type Args = RpcConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let client = ClientHandle::new()?;
        let scheduler = SchedulerHandle::new()?;

        scheduler
            .set_interval(actor_ref.downgrade(), TIMEOUT_CHECK_INTERVAL, TimeoutCheck)
            .await?;

        client.on_message().subscribe(actor_ref);

        Ok(Self {
            client,
            instance_name: config.instance_name,
            services: HashMap::new(),
            client_calls: Vec::new(),
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.services.clear();

        for call in &mut self.client_calls {
            call.on_actor_stop();
        }

        self.client_calls.clear();

        Ok(())
    }
}

impl message::Message<client::Message> for Rpc {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        for (_, service) in &self.services {
            service.clone().on_message(&msg);
        }

        for call in &mut self.client_calls {
            call.on_message(&msg);
        }

        self.clean_client_calls();
    }
}

#[derive(Debug, Error)]
pub enum RpcServiceAddError {
    #[error("service with address '{0}' does already exist")]
    AlreadyExists(String),
}

impl message::Message<ServiceAdd> for Rpc {
    type Reply = Result<(), RpcServiceAddError>;

    async fn handle(
        &mut self,
        mut msg: ServiceAdd,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.services.contains_key(&msg.address) {
            return Err(RpcServiceAddError::AlreadyExists(msg.address));
        }

        self.services.insert(
            msg.address.clone(),
            msg.service
                .create_handler(self.client.clone(), msg.address, &self.instance_name)
                .into(),
        );

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RpcServiceRemoveError {
    #[error("service with address '{0}' not found")]
    NotFound(String),
}

impl message::Message<ServiceRemove> for Rpc {
    type Reply = Result<(), RpcServiceRemoveError>;

    async fn handle(
        &mut self,
        msg: ServiceRemove,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.services.remove(&msg.address).is_none() {
            return Err(RpcServiceRemoveError::NotFound(msg.address));
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RpcCallError {
    #[error("cannot serialize request: {0}")]
    RequestSerializationError(#[source] serde_json::Error),
    #[error("cannot deserialize reply: {0}")]
    ReplyDeserializationError(#[source] serde_json::Error),
    #[error("remote error: {0}")]
    RemoteError(#[from] RemoteError),
    #[error("timeout during rpc call")]
    Timeout,
    #[error("rpc actor stopping during rpc call")]
    ActorStopping,
}

impl message::Message<Call> for Rpc {
    type Reply = DelegatedReply<Result<Value, RpcCallError>>;

    async fn handle(
        &mut self,
        msg: Call,
        ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let timeout = msg.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let (delegated_reply, reply_sender) = ctx.reply_sender();

        let mut call = match ClientCall::send(
            self.client.clone(),
            &self.instance_name,
            msg.target_instance,
            msg.address,
            msg.input,
            timeout,
        ) {
            Ok(call) => call,
            Err(error) => {
                if let Some(reply_sender) = reply_sender {
                    reply_sender.send(Err(error));
                }
                return delegated_reply;
            }
        };

        call.set_reply_sender(reply_sender);

        self.client_calls.push(call);

        delegated_reply
    }
}

impl message::Message<TimeoutCheck> for Rpc {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: TimeoutCheck,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        for call in &mut self.client_calls {
            call.on_timeout_check();
        }

        self.clean_client_calls();
    }
}

/// RPC server command: add service
#[derive(Debug)]
struct ServiceAdd {
    address: String,
    service: Box<dyn ServiceAddImpl>,
}

/// RPC server command: remove service
#[derive(Debug, Clone)]
struct ServiceRemove {
    address: String,
}

/// RPC client command: remote call
#[derive(Debug, Clone)]
struct Call {
    target_instance: String,
    address: String,
    input: Value,
    timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
struct TimeoutCheck;

/// Trait implemented by RPC service implementations
pub trait RpcService: Sync + Send {
    type Request;
    type Reply;

    fn handle(
        &self,
        request: Self::Request,
    ) -> impl Future<Output = anyhow::Result<Self::Reply>> + Send;
}

trait ServiceAddImpl: fmt::Debug + Send + Sync {
    fn create_handler(
        &mut self,
        client: ClientHandle,
        address: String,
        instance_name: &str,
    ) -> Box<dyn ServiceHandler>;
}

struct TypedServiceAddImpl<Request, Reply, Impl>(Option<Impl>)
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static;

impl<Request, Reply, Impl> TypedServiceAddImpl<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    pub fn new(implementation: Impl) -> Self {
        Self(Some(implementation))
    }
}

impl<Request, Reply, Impl> fmt::Debug for TypedServiceAddImpl<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypedServiceAddImpl")
            .finish_non_exhaustive()
    }
}

impl<Request, Reply, Impl> ServiceAddImpl for TypedServiceAddImpl<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    fn create_handler(
        &mut self,
        client: ClientHandle,
        address: String,
        instance_name: &str,
    ) -> Box<dyn ServiceHandler> {
        let implementation = self.0.take().expect("handler already created");
        Box::new(TypedServiceHandler::new(
            client,
            address,
            instance_name,
            implementation,
        ))
    }
}

#[async_trait]
trait ServiceHandler: Send + Sync {
    fn on_message(self: Arc<Self>, msg: &client::Message);
}

struct TypedServiceHandler<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    client: ClientHandle,
    address: String,
    topic: Topic,
    implementation: Impl,
}

impl<Request, Reply, Impl> Drop for TypedServiceHandler<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    fn drop(&mut self) {
        self.client.unsubscribe(self.topic.clone().into());
    }
}

#[async_trait]
impl<Request, Reply, Impl> ServiceHandler for TypedServiceHandler<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    fn on_message(self: Arc<Self>, msg: &client::Message) {
        if msg.topic() == self.topic.as_str() {
            let payload = msg.payload().clone();

            tokio::spawn(async move {
                self.handle(&payload).await;
            });
        }
    }
}

impl<Request, Reply, Impl> TypedServiceHandler<Request, Reply, Impl>
where
    Impl: RpcService<Request = Request, Reply = Reply> + 'static,
    Request: DeserializeOwned + 'static,
    Reply: Serialize + 'static,
{
    pub fn new(
        client: ClientHandle,
        address: String,
        instance_name: &str,
        implementation: Impl,
    ) -> Self {
        let topic = TopicBuilder::local(instance_name, DOMAIN)
            .segment(SERVICES)
            .segment(&address)
            .build();

        client.subscribe(topic.clone().into());

        Self {
            address,
            client,
            topic,
            implementation,
        }
    }

    async fn handle(&self, input: &Bytes) {
        let RpcRequest { input, reply_topic } = match serde_json::from_slice(input) {
            Ok(req) => req,
            Err(error) => {
                tracing::error!(
                    ?error,
                    address = self.address,
                    "could not deserialize request"
                );
                return;
            }
        };

        let reply = match self.handle_request(input).await {
            Ok(output) => RpcReply {
                output: Some(output),
                error: None,
            },
            Err(error) => RpcReply {
                output: None,
                error: Some(error.into()),
            },
        };

        let payload = match serde_json::to_vec(&reply) {
            Ok(payload) => Bytes::from_owner(payload),
            Err(error) => {
                tracing::error!(?error, address = self.address, "could not serialize reply");
                return;
            }
        };

        self.client
            .publish(Topic::from_raw(reply_topic), payload, false);
    }

    async fn handle_request(&self, input: Value) -> anyhow::Result<Value> {
        let request = serde_json::from_value::<Request>(input)?;
        let reply = self.implementation.handle(request).await?;
        let output = serde_json::to_value(reply)?;

        Ok(output)
    }
}

struct ClientCall {
    client: ClientHandle,
    reply_sender: Option<ReplySender<Result<Value, RpcCallError>>>,
    reply_topic: Topic,
    timeout: Instant,
    terminated: bool,
}

impl Drop for ClientCall {
    fn drop(&mut self) {
        self.client.unsubscribe(self.reply_topic.clone().into());
    }
}

impl ClientCall {
    pub fn send(
        client: ClientHandle,
        local_instance: &str,
        target_instance: String,
        address: String,
        input: Value,
        timeout: Duration,
    ) -> Result<ClientCall, RpcCallError> {
        let timeout = Instant::now()
            .checked_add(timeout)
            .expect("time math error");
        let call_topic = TopicBuilder::remote(&target_instance, DOMAIN)
            .segment(SERVICES)
            .segment(&address)
            .build();
        let reply_topic = TopicBuilder::local(local_instance, DOMAIN)
            .segment(REPLIES)
            .segment(&Self::random_topic_part())
            .build();

        let request = RpcRequest {
            input,
            reply_topic: reply_topic.to_string(),
        };

        let payload = Bytes::from_owner(
            serde_json::to_vec(&request).map_err(|e| RpcCallError::RequestSerializationError(e))?,
        );

        client.subscribe(reply_topic.clone().into());

        client.publish(call_topic, payload, false);

        Ok(Self {
            client,
            reply_sender: None,
            reply_topic,
            timeout,
            terminated: false,
        })
    }

    pub fn set_reply_sender(
        &mut self,
        reply_sender: Option<ReplySender<Result<Value, RpcCallError>>>,
    ) {
        self.reply_sender = reply_sender;
    }

    pub fn on_message(&mut self, msg: &client::Message) {
        if msg.topic() != self.reply_topic.as_str() {
            return;
        }

        let res = self.process_message(msg.payload());
        self.send_reply(res);
    }

    fn process_message(&self, payload: &Bytes) -> Result<Value, RpcCallError> {
        let reply: RpcReply = serde_json::from_slice(payload)
            .map_err(|e| RpcCallError::ReplyDeserializationError(e))?;

        if let Some(error) = reply.error {
            return Err(RpcCallError::RemoteError(RemoteError::from(error)));
        }

        let output = if let Some(output) = reply.output {
            output
        } else {
            // If we don't have a reply let's consider it's a null reply (but still successful)
            Value::Null
        };

        Ok(output)
    }

    pub fn on_timeout_check(&mut self) {
        if self.timeout < Instant::now() {
            self.send_reply(Err(RpcCallError::Timeout));
        }
    }

    pub fn on_actor_stop(&mut self) {
        self.send_reply(Err(RpcCallError::ActorStopping));
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    fn send_reply(&mut self, res: Result<Value, RpcCallError>) {
        if let Some(reply_sender) = self.reply_sender.take() {
            reply_sender.send(res);
        }

        self.terminated = true;
    }

    fn random_topic_part() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        const LEN: usize = 16;

        let mut rng = rand::rng();
        (0..LEN)
            .map(|_| *CHARSET.choose(&mut rng).unwrap() as char)
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcRequest {
    input: Value,
    reply_topic: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcReply {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcError {
    message: String,
    stacktrace: String,
}

impl From<anyhow::Error> for RpcError {
    fn from(error: anyhow::Error) -> Self {
        // anyhow::Error
        // - Display formats error message only
        // - Debug formats all the chain
        RpcError {
            message: format!("{}", error),
            stacktrace: format!("{:?}", error),
        }
    }
}

pub struct RemoteError {
    message: String,
    stacktrace: String,
}

impl From<RpcError> for RemoteError {
    fn from(value: RpcError) -> Self {
        Self {
            message: value.message,
            stacktrace: value.stacktrace,
        }
    }
}

impl std::error::Error for RemoteError {}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Debug for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.stacktrace)
    }
}
