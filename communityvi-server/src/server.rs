use crate::configuration::Configuration;
use crate::connection::split_websocket;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::unwind_safe_gotham_handler::UnwindSafeGothamHandler;
use gotham::hyper::http::{header, HeaderMap, Response};
use gotham::hyper::Body;
use gotham::hyper::StatusCode;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes, ScopeBuilder};
use gotham::router::Router;
use gotham::state::{FromState, State};
use log::error;

mod unwind_safe_gotham_handler;
mod websocket_upgrade;

pub type WebSocket = tokio_tungstenite::WebSocketStream<gotham::hyper::upgrade::Upgraded>;

pub async fn run_server(configuration: &Configuration, enable_reference_client: bool) {
	let room = Room::new(configuration.room_size_limit);
	let _ = gotham::init_server(configuration.address, create_router(room, enable_reference_client)).await;
}

pub fn create_router(room: Room, enable_reference_client: bool) -> Router {
	build_simple_router(move |route| {
		if enable_reference_client {
			route.scope("/reference", reference_client_scope);
		}

		route
			.get("/ws")
			.to_new_handler(UnwindSafeGothamHandler::from(move |state| {
				websocket_handler(room, state)
			}));
	})
}

fn reference_client_scope(route: &mut ScopeBuilder<(), ()>) {
	const REFERENCE_CLIENT_HTML: &str = include_str!("../static/reference.html");
	const REFERENCE_CLIENT_JAVASCRIPT: &str = include_str!("../static/reference.js");
	const REFERENCE_CLIENT_CSS: &str = include_str!("../static/reference.css");

	route.get("/").to(|state| {
		let response = Response::builder()
			.header(header::CONTENT_TYPE, mime::TEXT_HTML_UTF_8.to_string())
			.header(header::CACHE_CONTROL, "no-cache")
			// prevent XSS - FIXME: Make this work in Safari.
			.header(
				header::CONTENT_SECURITY_POLICY,
				"default-src 'none'; media-src 'self' blob:; img-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'",
			)
			.body(REFERENCE_CLIENT_HTML.into())
			.expect("Failed to build reference client HTML response");
		(state, response)
	});
	route.get("/reference.js").to(|state| {
		let response = Response::builder()
			.header(header::CONTENT_TYPE, mime::APPLICATION_JAVASCRIPT_UTF_8.to_string())
			.header(header::CACHE_CONTROL, "no-cache")
			.body(REFERENCE_CLIENT_JAVASCRIPT.into())
			.expect("Failed to build reference client JavaScript response");
		(state, response)
	});
	route.get("/reference.css").to(|state| {
		let response = Response::builder()
			.header(header::CONTENT_TYPE, mime::TEXT_CSS_UTF_8.to_string())
			.header(header::CACHE_CONTROL, "no-cache")
			.body(REFERENCE_CLIENT_CSS.into())
			.expect("Failed to build reference client CSS response");
		(state, response)
	});
}

fn websocket_handler(room: Room, mut state: State) -> (State, Response<Body>) {
	let body = Body::take_from(&mut state);
	let headers = HeaderMap::take_from(&mut state);
	let response = if websocket_upgrade::requested(&headers) {
		match websocket_upgrade::accept(&headers, body) {
			Ok((response, websocket_future)) => {
				tokio::spawn(async move {
					match websocket_future.await {
						Ok(websocket) => run_client_connection(room, websocket).await,
						Err(error) => error!("Failed to upgrade websocket with error {:?}.", error),
					}
				});
				response
			}
			Err(()) => bad_request(),
		}
	} else {
		bad_request()
	};
	(state, response)
}

fn bad_request() -> Response<Body> {
	Response::builder()
		.status(StatusCode::BAD_REQUEST)
		.body(Body::from("Bad Request"))
		.expect("Failed to build BAD_REQUEST response.")
}

async fn run_client_connection(room: Room, websocket: WebSocket) {
	let (message_sender, message_receiver) = split_websocket(websocket);
	run_client(room, message_sender, message_receiver).await
}
