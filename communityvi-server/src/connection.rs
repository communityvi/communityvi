use crate::connection::broadcast_buffer::BroadcastBuffer;
use crate::connection::sender::MessageSender;
use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use js_int::UInt;

pub mod broadcast_buffer;
pub mod receiver;
pub mod sender;

pub struct Connection {
	sender: MessageSender,
	broadcast_buffer: BroadcastBuffer,
}

impl Connection {
	pub fn new(sender: MessageSender, broadcast_buffer: BroadcastBuffer) -> Self {
		Self {
			sender,
			broadcast_buffer,
		}
	}

	pub async fn send_success_message(&self, message: SuccessMessage, request_id: UInt) -> bool {
		self.sender.send_success_message(message, request_id).await.is_ok()
	}

	pub async fn send_error_message(&self, message: ErrorMessage, request_id: Option<UInt>) -> bool {
		self.sender.send_error_message(message, request_id).await.is_ok()
	}

	pub async fn send_broadcast_message(&self, message: BroadcastMessage) -> bool {
		self.sender.send_broadcast_message(message).await.is_ok()
	}

	pub fn enqueue_broadcast(&self, message: BroadcastMessage, count: usize) {
		self.broadcast_buffer.enqueue(message, count);
	}

	pub async fn wait_for_broadcast(&self) -> BroadcastMessage {
		self.broadcast_buffer.wait_for_broadcast().await
	}

	pub async fn send_ping(&self, payload: Vec<u8>) -> bool {
		self.sender.send_ping(payload).await.is_ok()
	}
}

#[cfg(test)]
pub mod test {
	use crate::connection::receiver::ReceivedMessage;
	use crate::message::WebSocketMessage;
	use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
	use crate::utils::test_client::WebsocketTestClient;

	#[tokio::test]
	async fn should_close_after_10_invalid_messages() {
		let (_message_sender, mut message_receiver, mut test_client) = WebsocketTestClient::new();

		// send 10 invalid messages
		let invalid_message = WebSocketMessage::binary(vec![1u8, 2u8, 3u8, 4u8]);
		for _ in 0usize..10 {
			test_client.send_raw(invalid_message.clone()).await;
		}

		// try to receive them on the server
		assert_eq!(message_receiver.receive().await, ReceivedMessage::Finished);

		// receive 10 responses from the server
		for _ in 0usize..10 {
			test_client.receive_raw().await;
		}

		let too_many_retries_response = test_client.receive_error_message(None).await;
		assert_eq!(
			ErrorMessage::builder()
				.error(ErrorMessageType::InvalidOperation)
				.message("Too many retries".to_string())
				.build(),
			too_many_retries_response
		);

		let close_message = test_client.receive_raw().await;
		assert!(close_message.is_close());
	}
}
