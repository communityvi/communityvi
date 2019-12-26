use crate::message::{Message, OrderedMessage, WebSocketMessage};
use crate::room::{Client, Room};
use futures::compat::Future01CompatExt;
use futures::FutureExt;
use futures::TryFutureExt;
use futures01::future::join_all;
use futures01::sink::Sink;
use futures01::stream::Stream;
use futures01::{Future, IntoFuture};
use std::convert::Into;
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use warp::filters::ws::Ws2;
use warp::Filter;

pub fn create_server<ShutdownHandleType>(
	address: impl Into<SocketAddr> + 'static,
	shutdown_handle: ShutdownHandleType,
) -> impl std::future::Future<Output = ()>
where
	ShutdownHandleType: std::future::Future<Output = ()> + Send + 'static,
{
	let room = Arc::new(Room::default());

	let room_for_get = room.clone();
	let get_state = warp::get2().map(move || room_for_get.offset.load(Ordering::SeqCst).to_string());

	let state_for_post = room.clone();
	let post_state = warp::post2()
		.and(warp::path::param2())
		.map(move |new_offset| state_for_post.offset.store(new_offset, Ordering::SeqCst))
		.map(|_| http::response::Builder::new().status(204).body(""));

	let room_for_websocket = room.clone();
	let websocket_filter = warp::path("ws").and(warp::ws2()).map(move |ws2: Ws2| {
		let room = room_for_websocket.clone();
		ws2.on_upgrade(move |websocket| {
			let room = room;

			let (websocket_sink, websocket_stream) = websocket.split();
			let (message_sender, message_receiver) = futures01::sync::mpsc::channel::<OrderedMessage>(1);
			let client = room.add_client(message_sender.clone());
			let message_receive_future = message_receiver
				.map(WebSocketMessage::from)
				.forward(websocket_sink.sink_map_err(|_| ()))
				.map(|_| ());

			let stream_future = websocket_stream
				.take_while(|websocket_message| futures01::future::ok(!websocket_message.is_close()))
				.map_err(|error| eprintln!("Error streaming websocket messages: {}", error))
				.and_then(|websocket_message| {
					Message::try_from(websocket_message)
						.into_future()
						.map_err(|error| eprintln!("Error converting messages: {}", error))
				})
				.and_then(move |message| {
					let room = room.clone();
					handle_message(room, &client, message).unit_error().boxed().compat()
				})
				.for_each(|()| futures01::future::ok(()));

			let futures: Vec<Box<dyn Future<Item = (), Error = ()> + Send>> =
				vec![Box::new(message_receive_future), Box::new(stream_future)];
			join_all(futures).map(|_| ()).map_err(|_| ())
		})
	});

	let filter = websocket_filter.or(get_state).or(post_state);
	let server = warp::serve(filter);

	let compat_shutdown_handle = shutdown_handle.unit_error().boxed().compat();
	let (_address, future) = server.bind_with_graceful_shutdown(address, compat_shutdown_handle);
	future.compat().then(|_| futures::future::ready(()))
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
