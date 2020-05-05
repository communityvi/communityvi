use crate::connection::receiver::{MessageReceiver, WebSocketMessageReceiver};
use crate::connection::sender::{MessageSender, WebSocketMessageSender};
use crate::server::WebSocket;
use crate::utils::infallible_stream::InfallibleStream;
use futures::StreamExt;

pub mod receiver;
pub mod sender;

pub fn split_websocket(websocket: WebSocket) -> (MessageSender, MessageReceiver) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let websocket_client_connection = WebSocketMessageSender::new(websocket_sink);
	let client_connection = MessageSender::from(websocket_client_connection);
	let stream_server_connection =
		WebSocketMessageReceiver::new(InfallibleStream::from(websocket_stream), client_connection.clone());
	(client_connection, stream_server_connection.into())
}

#[cfg(test)]
pub mod test {
	use super::*;
	use crate::connection::receiver::StreamMessageReceiver;
	use crate::connection::sender::SinkMessageSender;
	use crate::message::client_request::ClientRequest;
	use crate::message::server_response::ServerResponse;
	use crate::message::{MessageError, WebSocketMessage};
	use crate::utils::test_client::TestClient;
	use futures::{Sink, SinkExt, Stream, StreamExt};
	use std::convert::TryFrom;
	use std::pin::Pin;

	pub type TypedTestClient = TestClient<
		Pin<Box<dyn Sink<ClientRequest, Error = futures::channel::mpsc::SendError>>>,
		Pin<Box<dyn Stream<Item = Result<ServerResponse, MessageError>>>>,
	>;
	pub type RawTestClient = TestClient<
		Pin<Box<dyn Sink<WebSocketMessage, Error = futures::channel::mpsc::SendError>>>,
		Pin<Box<dyn Stream<Item = WebSocketMessage>>>,
	>;

	pub fn create_typed_test_connections() -> (MessageSender, MessageReceiver, TypedTestClient) {
		let (client_connection, server_connection, raw_test_client) = create_raw_test_connections();
		let (raw_client_sender, raw_client_receiver) = raw_test_client.split();

		let client_sender = raw_client_sender.with(|client_request| {
			futures::future::ok::<_, futures::channel::mpsc::SendError>(WebSocketMessage::from(&client_request))
		});
		let client_receiver = raw_client_receiver.map(|websocket_message| ServerResponse::try_from(&websocket_message));

		(
			client_connection,
			server_connection,
			TestClient::new(Box::pin(client_sender), Box::pin(client_receiver)),
		)
	}

	pub fn create_raw_test_connections() -> (MessageSender, MessageReceiver, RawTestClient) {
		let (client_sender, server_receiver) = futures::channel::mpsc::unbounded();
		let (server_sender, client_receiver) = futures::channel::mpsc::unbounded();

		let sink_client_connection = SinkMessageSender::new(server_sender);
		let client_connection = MessageSender::from(sink_client_connection);
		let stream_server_connection = StreamMessageReceiver::new(server_receiver, client_connection.clone());

		let server_connection = MessageReceiver::from(stream_server_connection);

		(
			client_connection,
			server_connection,
			TestClient::new(Box::pin(client_sender), Box::pin(client_receiver)),
		)
	}

	#[tokio::test]
	async fn should_close_after_10_invalid_messages() {
		let (_client_connection, mut server_connection, mut test_client) = create_raw_test_connections();

		// send 10 invalid messages
		let invalid_message = WebSocketMessage::binary(vec![1u8, 2u8, 3u8, 4u8]);
		for _ in 0usize..10 {
			test_client.send(invalid_message.clone()).await;
		}

		// try to receive them on the server
		assert!(server_connection.receive().await.is_none());

		// receive 10 responses from the server
		for _ in 0usize..10 {
			test_client.receive().await;
		}

		let too_many_retries_response = test_client.receive().await;
		assert_eq!(
			WebSocketMessage::text(
				r#"{"type":"error","error":"invalid_operation","message":"Too many retries"}"#.to_string()
			),
			too_many_retries_response
		);

		let close_message = test_client.receive().await;
		assert!(close_message.is_close());
	}
}
