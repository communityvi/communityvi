use crate::connection::client::{ClientConnection, WebSocketClientConnection};
use crate::connection::server::{ServerConnection, WebSocketServerConnection};
use crate::infallible_stream::InfallibleStream;
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
