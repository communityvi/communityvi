use crate::connection::client::{ClientConnection, WebSocketClientConnection};
use crate::connection::server::{ServerConnection, WebSocketServerConnection};
use crate::utils::infallible_stream::InfallibleStream;
use futures::StreamExt;
use warp::filters::ws::WebSocket;

pub mod client;
pub mod server;

pub fn split_websocket(websocket: WebSocket) -> (ClientConnection, ServerConnection) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let websocket_client_connection = WebSocketClientConnection::new(websocket_sink);
	let stream_server_connection = WebSocketServerConnection::new(
		InfallibleStream::from(websocket_stream),
		websocket_client_connection.clone().into(),
	);
	(websocket_client_connection.into(), stream_server_connection.into())
}

#[cfg(test)]
pub mod test {
	use super::*;
	use crate::connection::client::SinkClientConnection;
	use crate::connection::server::StreamServerConnection;
	use crate::message::{ClientRequest, MessageError, OrderedMessage, ServerResponse, WebSocketMessage};
	use crate::utils::sink_stream::SinkStream;
	use futures::{Sink, SinkExt, Stream};
	use std::convert::TryFrom;
	use std::pin::Pin;

	pub type RawClientSinkStream = SinkStream<
		Pin<Box<dyn Sink<WebSocketMessage, Error = futures::channel::mpsc::SendError>>>,
		Pin<Box<dyn Stream<Item = WebSocketMessage>>>,
	>;
	pub type TypedClientSinkStream = SinkStream<
		Pin<Box<dyn Sink<OrderedMessage<ClientRequest>, Error = futures::channel::mpsc::SendError>>>,
		Pin<Box<dyn Stream<Item = Result<OrderedMessage<ServerResponse>, MessageError>>>>,
	>;

	pub fn create_typed_test_connections() -> (ClientConnection, ServerConnection, TypedClientSinkStream) {
		let (
			client_connection,
			server_connection,
			SinkStream {
				sink: client_sender,
				stream: client_receiver,
			},
		) = create_raw_test_connections();

		let client_sender = client_sender.with(|ordered_message| {
			futures::future::ok::<_, futures::channel::mpsc::SendError>(WebSocketMessage::from(&ordered_message))
		});
		let client_receiver =
			client_receiver.map(|websocket_message| OrderedMessage::<ServerResponse>::try_from(&websocket_message));

		(
			client_connection,
			server_connection,
			SinkStream::new(Box::pin(client_sender), Box::pin(client_receiver)),
		)
	}

	pub fn create_raw_test_connections() -> (ClientConnection, ServerConnection, RawClientSinkStream) {
		let (client_sender, server_receiver) = futures::channel::mpsc::unbounded();
		let (server_sender, client_receiver) = futures::channel::mpsc::unbounded();

		let sink_client_connection = SinkClientConnection::new(server_sender);
		let stream_server_connection =
			StreamServerConnection::new(server_receiver, sink_client_connection.clone().into());

		let client_connection = ClientConnection::from(sink_client_connection);
		let server_connection = ServerConnection::from(stream_server_connection);

		(
			client_connection,
			server_connection,
			SinkStream::new(Box::pin(client_sender), Box::pin(client_receiver)),
		)
	}

	#[tokio::test]
	async fn should_close_after_10_invalid_messages() {
		let (_client_connection, mut server_connection, mut client_sink_stream) = create_raw_test_connections();

		// send 10 invalid messages
		let invalid_message = WebSocketMessage::binary(vec![1u8, 2u8, 3u8, 4u8]);
		for _ in 0usize..10 {
			client_sink_stream
				.send(invalid_message.clone())
				.await
				.expect("Failed to send invalid message.");
		}

		// try to receive them on the server
		assert!(server_connection.receive().await.is_none());

		// receive 10 responses from the server
		for _ in 0usize..10 {
			client_sink_stream
				.next()
				.await
				.expect("Invalid websocket response received");
		}

		let too_many_retries_response = client_sink_stream.next().await.unwrap();
		assert_eq!(
			WebSocketMessage::text(
				r#"{"number":10,"type":"error","error":"invalid_operation","message":"Too many retries"}"#.to_string()
			),
			too_many_retries_response
		);

		let close_message = client_sink_stream.next().await.unwrap();
		assert!(close_message.is_close());
	}
}
