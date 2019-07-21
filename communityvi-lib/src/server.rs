use crate::room::Room;
use core::borrow::Borrow;
use futures::sink::Sink;
use futures::stream::Stream;
use futures::Future;
use std::convert::Into;
use std::net::SocketAddr;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use warp::filters::ws::{Message, Ws2};
use warp::Filter;

pub fn create_server<ShutdownHandleType>(
	address: impl Into<SocketAddr> + 'static,
	shutdown_handle: ShutdownHandleType,
) -> impl Future<Item = (), Error = ()>
where
	ShutdownHandleType: Future<Item = ()> + Send + 'static,
{
	let room = Arc::new(Room {
		offset: AtomicI64::new(42),
	});

	let state_for_get = room.clone();
	let get_state = warp::get2().map(move || state_for_get.offset.load(Ordering::SeqCst).to_string());

	let state_for_post = room.clone();
	let post_state = warp::post2()
		.and(warp::path::param2())
		.map(move |new_offset| state_for_post.offset.store(new_offset, Ordering::SeqCst))
		.map(|_| http::response::Builder::new().status(204).body(""));

	let room_for_websocket = room.clone();
	let websocket_filter = warp::path("ws").and(warp::ws2()).map(move |ws2: Ws2| {
		let state = room_for_websocket.clone();
		ws2.on_upgrade(move |websocket| {
			let state = state;
			let (sink, stream) = websocket.split();
			stream
				.take_while(|message| futures::future::ok(!message.is_close()))
				.map_err(|_| ())
				.and_then(move |message| {
					let state = state.borrow();
					handle_message(state, message)
				})
				.forward(sink.sink_map_err(|_| ()))
				.map(|_| ())
				.map_err(|_| ())
		})
	});

	let filter = websocket_filter.or(get_state).or(post_state);
	let server = warp::serve(filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future
}

fn handle_message(room: &Room, message: Message) -> impl Future<Item = Message, Error = ()> {
	println!("Message: {:?}", message);
	let offset = room.offset.load(Ordering::SeqCst).to_string();
	futures::future::ok(Message::text(offset))
}
