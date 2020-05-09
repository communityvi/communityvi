use crate::connection::sender::{MessageSender, MessageSenderTrait};
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
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
	async fn send_success_message(&self, _message: SuccessMessage, _request_id: u64) -> Result<(), ()> {
		Ok(())
	}

	async fn send_error_message(&self, _message: ErrorMessage, _request_id: Option<u64>) -> Result<(), ()> {
		Ok(())
	}

	async fn send_broadcast_message(&self, _message: BroadcastMessage) -> Result<(), ()> {
		Ok(())
	}

	async fn close(&self) {}
}
