use common::{bus::rpc::RpcService, utils::actors::CallError};

use crate::model::{ModelHandle, SetDefinitionError, definition::Definition};

#[derive(Debug)]
pub struct DefinitionSetRpcService(ModelHandle);

impl DefinitionSetRpcService {
    pub fn new(handle: ModelHandle) -> Self {
        Self(handle)
    }
}

impl RpcService for DefinitionSetRpcService {
    type Request = Definition;
    type Reply = ();
    type Error = CallError<SetDefinitionError>;

    async fn handle(&self, request: Self::Request) -> Result<Self::Reply, Self::Error> {
        self.0.set_definition(request).await
    }
}
