use gotham::hyper::http::{HeaderMap, Response};
use gotham::hyper::upgrade::OnUpgrade;
use gotham::hyper::Body;
use gotham::hyper::StatusCode;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes, RouterBuilder};
use gotham::router::Router;
use gotham::state::{FromState, State};
use log::error;

use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::unwind_safe_gotham_handler::UnwindSafeGothamHandler;
use futures::{SinkExt, StreamExt, TryStreamExt};

mod etag;
mod file_bundle;
mod unwind_safe_gotham_handler;
mod websocket_upgrade;

pub async fn run_gotham_server(application_context: &ApplicationContext) {
	let room = Room::new(application_context.configuration.room_size_limit);
	let _ = gotham::init_server(
		application_context.configuration.address,
		create_router(application_context.clone(), room),
	)
	.await;
}

#[cfg(not(feature = "bundle-frontend"))]
#[allow(clippy::pedantic)]
pub async fn run_rweb_server(_application_context: &ApplicationContext) {}

#[cfg(feature = "bundle-frontend")]
pub async fn run_rweb_server(application_context: &ApplicationContext) {
	use crate::server::file_bundle::BundledFileHandler;
	use include_dir::{include_dir, Dir};
	use rweb::path::Tail;
	use rweb::{filters, Filter};
	use std::sync::Arc;

	const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");

	let bundled_file_handler = Arc::new(BundledFileHandler::from(FRONTEND_BUILD));

	let client_files = filters::path::tail()
		.and(filters::header::headers_cloned())
		.map(move |path: Tail, headers: HeaderMap| bundled_file_handler.handle_request(path.as_str(), &headers));

	rweb::serve(client_files)
		.run(application_context.configuration.address)
		.await
}

pub fn create_router(application_context: ApplicationContext, room: Room) -> Router {
	build_simple_router(move |route| {
		route.get("/ws").to_new_handler(UnwindSafeGothamHandler::from({
			let application_context = application_context.clone();
			move |state| websocket_handler(application_context, room, state)
		}));
		add_frontend_handler(route);
	})
}

#[cfg(feature = "bundle-frontend")]
fn add_frontend_handler(route: &mut RouterBuilder<(), ()>) {
	use file_bundle::BundledFileHandler;
	use include_dir::{include_dir, Dir};

	const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");

	let new_handler = || Ok(BundledFileHandler::from(FRONTEND_BUILD));
	route.get("/*").to_new_handler(new_handler);
	route.get("/").to_new_handler(new_handler);
}

#[cfg(not(feature = "bundle-frontend"))]
fn add_frontend_handler(_route: &mut RouterBuilder<(), ()>) {}

fn websocket_handler(application_context: ApplicationContext, room: Room, mut state: State) -> (State, Response<Body>) {
	let headers = HeaderMap::take_from(&mut state);
	let on_upgrade = OnUpgrade::try_take_from(&mut state);
	let response = match on_upgrade {
		Some(on_upgrade) if websocket_upgrade::requested(&headers) => {
			match websocket_upgrade::accept(&headers, on_upgrade) {
				Ok((response, websocket_future)) => {
					tokio::spawn(async move {
						match websocket_future.await {
							Ok(websocket) => {
								let (websocket_sink, websocket_stream) = websocket.split();
								let message_sender = MessageSender::from(websocket_sink.sink_map_err(Into::into));
								let message_receiver =
									MessageReceiver::new(websocket_stream.map_err(Into::into), message_sender.clone());
								let (message_sender, message_receiver) = (message_sender, message_receiver);
								run_client(application_context, room, message_sender, message_receiver).await
							}
							Err(error) => error!("Failed to upgrade websocket with error {:?}.", error),
						}
					});
					response
				}
				Err(()) => bad_request(),
			}
		}
		_ => bad_request(),
	};
	(state, response)
}

fn bad_request() -> Response<Body> {
	Response::builder()
		.status(StatusCode::BAD_REQUEST)
		.body(Body::from("Bad Request"))
		.expect("Failed to build BAD_REQUEST response.")
}
