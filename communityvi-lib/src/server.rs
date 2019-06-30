use crate::state::State;
use futures::stream::Stream;
use futures::Future;
use std::convert::Into;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use warp::filters::ws::Ws2;
use warp::Filter;

pub fn create_server<ShutdownHandleType>(
	address: impl Into<SocketAddr> + 'static,
	shutdown_handle: ShutdownHandleType,
) -> impl Future<Item = (), Error = ()>
where
	ShutdownHandleType: Future<Item = ()> + Send + 'static,
{
	let state = Arc::new(State {
		offset: AtomicU64::new(42),
	});

	let state_for_get = state.clone();
	let get_state = warp::get2().map(move || state_for_get.offset.load(Ordering::SeqCst).to_string());

	let state_for_post = state.clone();
	let post_state = warp::post2()
		.and(warp::path::param2())
		.map(move |new_offset| state_for_post.offset.store(new_offset, Ordering::SeqCst))
		.map(|_| http::response::Builder::new().status(204).body(""));

	let websocket_filter = warp::path("ws").and(warp::ws2()).map(|ws2: Ws2| {
		ws2.on_upgrade(|websocket| {
			let (sink, stream) = websocket.split();
			stream
				.inspect(|message| println!("{:?}", message))
				.take_while(|message| futures::future::ok(!message.is_close()))
				.forward(sink)
				.map(|_| ())
				.map_err(|error| {
					eprintln!("{}", error);
				})
		})
	});

	let filter = websocket_filter.or(get_state).or(post_state);
	let server = warp::serve(filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future
}
