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
	use futures::{Sink, SinkExt, Stream};
	use std::convert::TryFrom;

	pub fn test_client_connection() -> (
		ClientConnection,
		impl Stream<Item = Result<OrderedMessage<ServerResponse>, MessageError>>,
		impl Sink<OrderedMessage<ClientRequest>>,
	) {
		let (client_sender, server_receiver) = futures::channel::mpsc::unbounded();
		let (server_sender, client_receiver) = futures::channel::mpsc::unbounded();

		let sink_client_connection = SinkClientConnection::new(server_sender);
		let stream_server_connection =
			StreamServerConnection::new(server_receiver, sink_client_connection.clone().into());

		let server_response_stream = client_receiver.map(move |websocket_message: WebSocketMessage| {
			// Crazy Hack to Move the ownership into the server_response_stream so that it can
			// escape this function otherwise it would get dropped and the connection would break
			let _keep_server_connection_alive = &stream_server_connection;

			OrderedMessage::<ServerResponse>::try_from(&websocket_message)
		});
		let client_request_sink = client_sender.with(|ordered_message| {
			futures::future::ok::<_, futures::channel::mpsc::SendError>(WebSocketMessage::from(&ordered_message))
		});

		(
			sink_client_connection.into(),
			server_response_stream,
			client_request_sink,
		)
	}
}
