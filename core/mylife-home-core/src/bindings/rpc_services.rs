use common::{bus::rpc::RpcService, utils::actors::CallError};

use crate::bindings::{BindingAddError, BindingConfig, BindingRemoveError, BindingsHandle};

#[derive(Debug)]
pub struct BindingAddRpcService(BindingsHandle);

impl BindingAddRpcService {
    pub fn new(handle: BindingsHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for BindingAddRpcService {
    type Request = BindingConfig;
    type Reply = ();
    type Error = CallError<BindingAddError>;

    async fn handle(&self, request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.binding_add(request).await
    }
}

#[derive(Debug)]
pub struct BindingRemoveRpcService(BindingsHandle);

impl BindingRemoveRpcService {
    pub fn new(handle: BindingsHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for BindingRemoveRpcService {
    type Request = BindingConfig;
    type Reply = ();
    type Error = CallError<BindingRemoveError>;

    async fn handle(&self, request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.binding_remove(request).await
    }
}

#[derive(Debug)]
pub struct BindingListRpcService(BindingsHandle);

impl BindingListRpcService {
    pub fn new(handle: BindingsHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for BindingListRpcService {
    type Request = ();
    type Reply = Vec<BindingConfig>;
    type Error = CallError<CallError>;

    async fn handle(&self, _request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.binding_list().await
    }
}
