use common::{bus::rpc::RpcService, utils::actors::CallError};
use serde::Deserialize;

use crate::components::{
    ComponentConfig, LocalComponentAddError, LocalComponentRemoveError, LocalComponentsHandle,
};

#[derive(Debug)]
pub struct ComponentAddRpcService(LocalComponentsHandle);

impl ComponentAddRpcService {
    pub fn new(handle: LocalComponentsHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for ComponentAddRpcService {
    type Request = ComponentConfig;
    type Reply = ();
    type Error = CallError<LocalComponentAddError>;

    async fn handle(&self, request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0
            .component_add(request.id, request.plugin, request.config)
            .await
    }
}

#[derive(Debug)]
pub struct ComponentRemoveRpcService(LocalComponentsHandle);

impl ComponentRemoveRpcService {
    pub fn new(handle: LocalComponentsHandle) -> Self {
        Self(handle)
    }
}

#[derive(Debug, Deserialize)]
pub struct ComponentRemoveRequest {
    id: String,
}

impl RpcService for ComponentRemoveRpcService {
    type Request = ComponentRemoveRequest;
    type Reply = ();
    type Error = CallError<LocalComponentRemoveError>;

    async fn handle(&self, request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.component_remove(request.id).await
    }
}

#[derive(Debug)]
pub struct ComponentListRpcService(LocalComponentsHandle);

impl ComponentListRpcService {
    pub fn new(handle: LocalComponentsHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for ComponentListRpcService {
    type Request = ();
    type Reply = Vec<ComponentConfig>;
    type Error = CallError;

    async fn handle(&self, _request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.component_list().await
    }
}
