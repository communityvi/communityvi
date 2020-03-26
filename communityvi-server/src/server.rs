use crate::client::Client;
use crate::message::{ClientRequest, OrderedMessage, ServerResponse, WebSocketMessage};
use crate::room::Room;
use futures::future::join;
use futures::future::join_all;
use futures::{FutureExt, SinkExt, Stream};
use futures::{StreamExt, TryStreamExt};
use log::{debug, error, info};
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
			let reply = ws.on_upgrade(move |websocket| async move {
				let (websocket_sink, websocket_stream) = websocket.split();
				let (message_sender, message_receiver) =
					futures::channel::mpsc::channel::<OrderedMessage<ServerResponse>>(1);
				let client = room.add_client(message_sender);
				let message_receive_future = message_receiver
					.map(|message| WebSocketMessage::from(&message))
					.map(Ok)
					.forward(websocket_sink.sink_map_err(|_| ()))
					.map(|_| ());

				let stream_future = receive_messages(websocket_stream, client.clone(), &room);

				// type erasure for faster compile times!
				let handle_messages_and_respond: Pin<Box<dyn Future<Output = ()> + Send>> =
					Box::pin(join(message_receive_future, stream_future).map(|_| ()));
				handle_messages_and_respond.await;

				room.clients.remove(&client);
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
	room: &Room,
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

		let OrderedMessage { number, message } = match OrderedMessage::<ClientRequest>::try_from(websocket_message) {
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

		handle_message(room, &client, message).await
	}
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => room.singlecast(&client, ServerResponse::Pong).await,
		ClientRequest::Chat { message } => room.broadcast(ServerResponse::Chat { message }).await,
		ClientRequest::Pong => info!("Received Pong from client: {}", client.id()),
	}
}
