use crate::state::State;
use futures::Future;
use http::Response;
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
	let state = State {
		offset: Arc::new(AtomicU64::new(42)),
	};

	let offset_for_get = state.offset.clone();
	let get_state = warp::get2().map(move || offset_for_get.load(Ordering::Relaxed).to_string());
	let offset_for_post = state.offset.clone();
	let post_state = warp::post2()
		.and(warp::path::param2())
		.map(move |new_offset| offset_for_post.store(new_offset, Ordering::Relaxed))
		.map(|_| http::response::Builder::new().status(204).body(""));

	let filter = get_state.or(post_state);
	let server = warp::serve(filter);

	let (address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
	future
}
