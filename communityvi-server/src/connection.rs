use crate::message::{ClientRequest, OrderedMessage, ServerResponse, WebSocketMessage};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::error;
use std::sync::Arc;
use warp::filters::ws::WebSocket;

pub fn split_websocket(websocket: WebSocket) -> (ClientConnection, ServerConnection) {
	let (websocket_sink, websocket_stream) = websocket.split();
	let client_connection = ClientConnection {
		websocket_sink: Arc::new(tokio::sync::Mutex::new(websocket_sink)),
	};
	let server_connection = ServerConnection { websocket_stream };
	(client_connection, server_connection)
}

pub struct ServerConnection {
	websocket_stream: SplitStream<WebSocket>,
}

impl ServerConnection {
	/// Receive a message from the client or None if the connection has been closed.
	pub async fn receive(&mut self) -> Option<OrderedMessage<ClientRequest>> {
		let websocket_message = match self.websocket_stream.next().await {
			Some(Ok(websocket_message)) => websocket_message,
			Some(Err(error)) => {
				error!("Error streaming websocket message: {}, result.", error);
				return None;
			}
			None => return None,
		};
		Some(OrderedMessage::from(websocket_message))
	}
}

#[derive(Clone, Debug)]
pub struct ClientConnection {
	websocket_sink: Arc<tokio::sync::Mutex<SplitSink<WebSocket, WebSocketMessage>>>,
}

impl ClientConnection {
	pub async fn send(&self, message: OrderedMessage<ServerResponse>) -> Result<(), ()> {
		let mut sink = self.websocket_sink.lock().await;
		let websocket_message = WebSocketMessage::from(&message);
		sink.send(websocket_message).await.map_err(|_| ())
	}
}
