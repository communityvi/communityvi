use crate::connection::client::ClientConnection;
use crate::connection::server::ServerConnection;
use futures::StreamExt;
use warp::filters::ws::WebSocket;

pub mod client;
pub mod server;

pub fn split_websocket(websocket: WebSocket) -> (ClientConnection, ServerConnection) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let client_connection = ClientConnection::new(websocket_sink);
	let server_connection = ServerConnection::new(websocket_stream, client_connection.clone());
	(client_connection, server_connection)
}
