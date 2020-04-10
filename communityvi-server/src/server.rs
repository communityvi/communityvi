use crate::lifecycle::run_client;
use crate::room::Room;
use futures::FutureExt;
use log::info;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use warp::filters::ws::Ws;
use warp::{Filter, Rejection, Reply};

const REFERENCE_CLIENT_HTML: &str = include_str!("../static/reference.html");
const REFERENCE_CLIENT_JAVASCRIPT: &str = include_str!("../static/reference.js");

pub async fn create_server<ShutdownHandleType>(
	address: SocketAddr,
	shutdown_handle: ShutdownHandleType,
	enable_reference_client: bool,
) where
	ShutdownHandleType: std::future::Future<Output = ()> + Send + 'static,
{
	let room = Arc::new(Room::default());
	let websocket_filter = warp::path("ws")
		.boxed()
		.and(warp::path::end().boxed())
		.and(warp::ws().boxed())
		.and_then(move |ws: Ws| {
			let room = room.clone();
			let reply = ws.on_upgrade(move |websocket| async move { run_client(&room, websocket).await }.boxed());
			Box::pin(async { Ok(Box::new(reply) as Box<dyn Reply>) })
				as Pin<Box<dyn Future<Output = Result<Box<dyn Reply>, Rejection>> + Send>> // type erasure for faster compile times!
		})
		.boxed();

	let reference_client_html_filter = warp::get()
		.and(warp::path("reference"))
		.and(warp::path::end())
		.map(|| {
			warp::http::Response::builder()
			.header("Content-Type", "text/html; charset=utf-8")
			.header("Cache-Control", "no-cache") 
			// prevent XSS - FIXME: Make this work in Safari.
			.header(
				"Content-Security-Policy",
				"default-src 'none'; img-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'",
			)
			.body(REFERENCE_CLIENT_HTML)
		})
		.boxed();

	let reference_client_javascript_filter = warp::get()
		.and(warp::path("reference"))
		.and(warp::path("reference.js"))
		.and(warp::path::end())
		.map(|| {
			warp::http::Response::builder()
				.header("Content-Type", "application/javascript; charset=utf-8")
				.header("Cache-Control", "no-cache")
				.body(REFERENCE_CLIENT_JAVASCRIPT)
		})
		.boxed();

	let reference_client_filter = reference_client_html_filter.or(reference_client_javascript_filter);

	let (bound_address, future) = if enable_reference_client {
		let complete_filter = websocket_filter.or(reference_client_filter);
		let server = warp::serve(complete_filter);
		let (bound_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
		(bound_address, future.boxed())
	} else {
		let server = warp::serve(websocket_filter);
		let (bound_address, future) = server.bind_with_graceful_shutdown(address, shutdown_handle);
		(bound_address, future.boxed())
	};
	info!("Listening on {}", bound_address);
	future.await
}
