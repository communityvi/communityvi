use crate::message::{ClientRequest, Message, ServerResponse, WebSocketMessage};
use crate::room::{Client, Room};
use futures::future::join;
use futures::future::join_all;
use futures::{FutureExt, SinkExt, Stream};
use futures::{StreamExt, TryStreamExt};
use log::{debug, error};
use std::convert::Into;
use std::convert::TryFrom;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use warp::filters::ws::Ws;
use warp::{Filter, Rejection, Reply};

pub async fn create_server<ShutdownHandleType>(
	address: impl Into<SocketAddr> + 'static,
	shutdown_handle: ShutdownHandleType,
) where
	ShutdownHandleType: std::future::Future<Output = ()> + Send + 'static,
{
	let room = Arc::new(Room::default());
	let websocket_filter = warp::path("ws")
		.and(warp::path::end())
		.and(warp::ws())
		.and_then(move |ws: Ws| {
			let room = room.clone();
			let reply = ws.on_upgrade(move |websocket| {
				let (websocket_sink, websocket_stream) = websocket.split();
				let (message_sender, message_receiver) = futures::channel::mpsc::channel::<Message<ServerResponse>>(1);
				let client = room.add_client(message_sender);
				let message_receive_future = message_receiver
					.map(|message| WebSocketMessage::from(&message))
					.map(Ok)
					.forward(websocket_sink.sink_map_err(|_| ()))
					.map(|_| ());

				let stream_future = receive_messages(websocket_stream, client, room);

				Box::pin(join(message_receive_future, stream_future).map(|_| ()))
					as Pin<Box<dyn Future<Output = ()> + Send>> // type erasure for faster compile times!
			});
			Box::pin(async { Ok(Box::new(reply) as Box<dyn Reply>) })
				as Pin<Box<dyn Future<Output = Result<Box<dyn Reply>, Rejection>> + Send>> // type erasure for faster compile times!
		});

	let server = warp::serve(websocket_filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future.await
}

async fn receive_messages(
	mut websocket_stream: impl Stream<Item = Result<warp::ws::Message, warp::Error>> + Unpin + Send,
	client: Client,
	room: Arc<Room>,
) {
	loop {
		let websocket_message = match websocket_stream.next().await {
			Some(Ok(message)) => message,
			Some(Err(error)) => {
				error!("Error streaming websocket messages: {}", error);
				return; // stop reading from websocket when it is broken
			}
			None => return, // websocket has been closed
		};

		let Message { number, message } = match Message::<ClientRequest>::try_from(websocket_message) {
			Ok(message) => message,
			Err(error) => {
				error!("Error converting messages: {}", error);
				continue; // single message wasn't deserializable, continue with next one
			}
		};
		debug!(
			"Received {:?} message {} from client {}",
			std::mem::discriminant(&message),
			number,
			client.id()
		);

		handle_message(room.as_ref(), &client, message).await
	}
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => room.singlecast(&client, ServerResponse::Pong).await,
		ClientRequest::Chat { message } => room.broadcast(ServerResponse::Chat { message }).await,
		_ => unimplemented!(),
	}
}
