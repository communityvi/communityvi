pub mod broadcast_buffer;
pub mod receiver;
pub mod sender;

#[cfg(test)]
pub mod test {
	use crate::connection::receiver::ReceivedMessage;
	use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
	use crate::message::WebSocketMessage;
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
