use crate::state::State;
use futures::Future;
use std::convert::Into;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use warp::Filter;

pub fn create_server<ShutdownHandleType>(
	address: impl Into<SocketAddr>,
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

	let filter = get_state.or(post_state);
	let server = warp::serve(filter);

	let (_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future
}
