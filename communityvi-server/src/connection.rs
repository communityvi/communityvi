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

	pub fn create_typed_test_connections() -> (
		ClientConnection,
		ServerConnection,
		SinkStream<
			impl Sink<OrderedMessage<ClientRequest>, Error = futures::channel::mpsc::SendError>,
			impl Stream<Item = Result<OrderedMessage<ServerResponse>, MessageError>>,
		>,
	) {
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
			SinkStream::new(client_sender, client_receiver),
		)
	}

	pub fn create_raw_test_connections() -> (
		ClientConnection,
		ServerConnection,
		SinkStream<
			impl Sink<WebSocketMessage, Error = futures::channel::mpsc::SendError>,
			impl Stream<Item = WebSocketMessage>,
		>,
	) {
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
			SinkStream::new(client_sender, client_receiver),
		)
	}
}
