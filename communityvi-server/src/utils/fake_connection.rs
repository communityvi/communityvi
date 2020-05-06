use crate::connection::sender::{MessageSender, MessageSenderTrait};
use crate::message::broadcast::Broadcast;
use crate::message::server_response::ServerResponse;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct FakeClientConnection {}

impl From<FakeClientConnection> for MessageSender {
	fn from(fake_client_connection: FakeClientConnection) -> Self {
		Arc::pin(fake_client_connection)
	}
}

#[async_trait]
impl MessageSenderTrait for FakeClientConnection {
	async fn send(&self, _message: ServerResponse) -> Result<(), ()> {
		Ok(())
	}

	async fn send_broadcast_message(&self, _message: Broadcast) -> Result<(), ()> {
		Ok(())
	}

	async fn close(&self) {}
}
