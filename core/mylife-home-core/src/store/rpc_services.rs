use common::{bus::rpc::RpcService, utils::actors::CallError};

use crate::store::{SaveError, StoreHandle};

#[derive(Debug)]
pub struct SaveRpcService(StoreHandle);

impl SaveRpcService {
    pub fn new(handle: StoreHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for SaveRpcService {
    type Request = ();
    type Reply = ();
    type Error = CallError<SaveError>;

    async fn handle(&self, _request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.save().await
    }
}
