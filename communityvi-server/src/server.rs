use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes, RouterBuilder};
use gotham::router::Router;
use gotham::state::{FromState, State};
use log::error;
use rweb::http::{HeaderMap, Response};
use rweb::hyper::upgrade::OnUpgrade;
use rweb::hyper::Body;
use rweb::hyper::StatusCode;

use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::unwind_safe_gotham_handler::UnwindSafeGothamHandler;
use crate::utils::websocket_message_conversion::{
	rweb_websocket_message_to_tungstenite_message, tungstenite_message_to_rweb_websocket_message,
};
use futures::{SinkExt, StreamExt, TryStreamExt};
use rweb::warp::filters::BoxedFilter;
use rweb::{Filter, Reply};
use std::future::ready;

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

pub async fn run_rweb_server(application_context: ApplicationContext) {
	let address = application_context.configuration.address;
	rweb::serve(websocket_filter(application_context).or(bundled_frontend_filter()))
		.run(address)
		.await;
}

fn websocket_filter(application_context: ApplicationContext) -> BoxedFilter<(impl Reply,)> {
	let room = Room::new(application_context.configuration.room_size_limit);

	rweb::path("ws")
		.and(rweb::ws())
		.map(
			move |ws: rweb::ws::Ws /*, room: Room, application_context: ApplicationContext*/| {
				let room = room.clone();
				let application_context = application_context.clone();

				ws.on_upgrade(move |websocket| {
					let (sink, stream) = websocket.split();

					let message_sender = MessageSender::from(
						sink.with(|message| {
							ready(Ok::<_, anyhow::Error>(tungstenite_message_to_rweb_websocket_message(
								message,
							)))
						})
						.sink_map_err(Into::into),
					);
					let message_receiver = MessageReceiver::new(
						stream
							.map_ok(rweb_websocket_message_to_tungstenite_message)
							.map_err(Into::into),
						message_sender.clone(),
					);

					run_client(application_context, room, message_sender, message_receiver)
				})
			},
		)
		.boxed()
}

#[cfg(feature = "bundle-frontend")]
fn bundled_frontend_filter() -> BoxedFilter<(Response<Body>,)> {
	use crate::server::file_bundle::BundledFileHandler;
	use include_dir::{include_dir, Dir};
	use rweb::filters;
	use rweb::path::Tail;
	use std::sync::Arc;

	const FRONTEND_BUILD: Dir = include_dir!("../communityvi-frontend/build");

	let bundled_file_handler = Arc::new(BundledFileHandler::from(FRONTEND_BUILD));

	filters::path::tail()
		.and(filters::header::headers_cloned())
		.map(move |path: Tail, headers: HeaderMap| bundled_file_handler.handle_request(path.as_str(), &headers))
		.boxed()
}

#[cfg(not(feature = "bundle-frontend"))]
fn bundled_frontend_filter() -> BoxedFilter<(Response<Body>,)> {
	rweb::any().and_then(|| ready(Err(rweb::reject::not_found()))).boxed()
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
								run_client(application_context, room, message_sender, message_receiver).await;
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
