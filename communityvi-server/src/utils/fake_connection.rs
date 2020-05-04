use crate::connection::client::{ClientConnection, ClientConnectionTrait};
use crate::message::server_response::ServerResponse;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct FakeClientConnection {}

impl From<FakeClientConnection> for ClientConnection {
	fn from(fake_client_connection: FakeClientConnection) -> Self {
		Arc::pin(fake_client_connection)
	}
}

#[async_trait]
impl ClientConnectionTrait for FakeClientConnection {
	async fn send(&self, _message: ServerResponse) -> Result<(), ()> {
		Ok(())
	}

	async fn close(&self) {}
}
