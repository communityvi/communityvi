use crate::message::{Message, OrderedMessage, WebSocketMessage};
use crate::room::{Client, Room};
use futures::future::join;
use futures::future::join_all;
use futures::{FutureExt, Sink, SinkExt, Stream};
use futures::{StreamExt, TryStreamExt};
use log::error;
use std::convert::TryFrom;
use std::convert::{Infallible, Into};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use warp::filters::ws::Ws;
use warp::{Filter, Rejection};

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
				let (message_sender, message_receiver) = futures::channel::mpsc::channel::<OrderedMessage>(1);
				let client = room.add_client(message_sender.clone());
				let message_receive_future = message_receiver
					.map(WebSocketMessage::from)
					.map(Ok)
					.forward(websocket_sink.sink_map_err(|_| ()))
					.map(|_| ());

				let stream_future = receive_messages(websocket_stream, client, room);

				// erase the types because otherwise the compiler can't handle the nested types anymore
				let message_receive_future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(message_receive_future);
				let stream_future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(stream_future);
				join(message_receive_future, stream_future).map(|_| ())
			});
			futures::future::ok::<_, Rejection>(reply)
		});

	let server = warp::serve(websocket_filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future.await
}

async fn receive_messages(
	mut websocket_stream: impl Stream<Item = Result<warp::ws::Message, warp::Error>> + Unpin,
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

		let message = match Message::try_from(websocket_message) {
			Ok(message) => message,
			Err(error) => {
				error!("Error converting messages: {}", error);
				continue; // single message wasn't deserializable, continue with next one
			}
		};

		handle_message(room.as_ref(), &client, message).await
	}
}

async fn handle_message(room: &Room, client: &Client, message: Message) {
	match message {
		Message::Ping(text_message) => room.singlecast(&client, Message::Pong(text_message)).await,
		Message::Chat(text_message) => room.broadcast(Message::Chat(text_message)).await,
		_ => unimplemented!(),
	}
}
