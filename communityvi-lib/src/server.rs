use crate::message::{Message, OrderedMessage, WebSocketMessage};
use crate::room::{Client, Room};
use futures::future::join_all;
use futures::{FutureExt, SinkExt};
use futures::{StreamExt, TryStreamExt};
use std::convert::Into;
use std::convert::TryFrom;
use std::net::SocketAddr;
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

	let room_for_get = room.clone();
	let get_state = warp::get()
		.and(warp::path::end())
		.map(move || room_for_get.offset.load(Ordering::SeqCst).to_string());

	let state_for_post = room.clone();
	let post_state = warp::post()
		.and(warp::path::param())
		.and(warp::path::end())
		.map(move |new_offset| state_for_post.offset.store(new_offset, Ordering::SeqCst))
		.map(|_| http::response::Builder::new().status(204).body("").unwrap());

	let room_for_websocket = room.clone();
	let websocket_filter = warp::path("ws")
		.and(warp::path::end())
		.and(warp::ws())
		.and_then(move |ws: Ws| {
			let room = room_for_websocket.clone();
			let reply = ws.on_upgrade(move |websocket| {
				let room = room;

				let (websocket_sink, websocket_stream) = websocket.split();
				let (message_sender, message_receiver) = futures::channel::mpsc::channel::<OrderedMessage>(1);
				let client = room.add_client(message_sender.clone());
				let message_receive_future = message_receiver
					.map(WebSocketMessage::from)
					.map(Ok)
					.forward(websocket_sink.sink_map_err(|_| ()))
					.map(|_| ());

				let stream_future = websocket_stream
					.take_while(|websocket_message_result| {
						let continue_on = websocket_message_result
							.as_ref()
							.map(|message| !message.is_close())
							.unwrap_or(false);
						futures::future::ready(continue_on)
					})
					.map_err(|error| eprintln!("Error streaming websocket messages: {}", error))
					.and_then(|websocket_message| {
						async {
							Message::try_from(websocket_message)
								.map_err(|error| eprintln!("Error converting messages: {}", error))
						}
					})
					.and_then(move |message| {
						let room = room.clone();
						handle_message(room, &client, message).unit_error()
					})
					.for_each(|_: Result<(), ()>| futures::future::ready(()));

				let futures: Vec<Box<dyn std::future::Future<Output = ()> + Send + Unpin>> = vec![
					Box::new(message_receive_future.boxed()),
					Box::new(stream_future.boxed()),
				];
				join_all(futures).map(|_: Vec<()>| ())
			});
			futures::future::ok::<_, Rejection>(reply)
		});

	let filter = websocket_filter.or(get_state).or(post_state);
	let server = warp::serve(filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future.await
}

fn handle_message(room: Arc<Room>, client: &Client, message: Message) -> impl std::future::Future<Output = ()> {
	let client = client.clone();
	async move {
		match message {
			Message::Ping(text_message) => room.singlecast(&client, Message::Pong(text_message)).await,
			Message::Chat(text_message) => room.broadcast(Message::Chat(text_message)).await,
			_ => unimplemented!(),
		}
	}
}
