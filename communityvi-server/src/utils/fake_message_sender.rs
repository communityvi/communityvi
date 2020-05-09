use crate::connection::sender::{MessageSender, MessageSenderTrait};
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct FakeMessageSender {}

impl From<FakeMessageSender> for MessageSender {
	fn from(fake_message_sender: FakeMessageSender) -> Self {
		Arc::pin(fake_message_sender)
	}
}

#[async_trait]
impl MessageSenderTrait for FakeMessageSender {
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
