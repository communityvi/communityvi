use rweb::http::Response;
use rweb::hyper::Body;

use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::utils::websocket_message_conversion::{
	rweb_websocket_message_to_tungstenite_message, tungstenite_message_to_rweb_websocket_message,
};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use rweb::warp::filters::BoxedFilter;
use rweb::{Filter, Reply};
use std::future::ready;

mod file_bundle;
mod rest_api;

pub async fn run_server(application_context: ApplicationContext) {
	let room = Room::new(application_context.configuration.room_size_limit);
	let address = application_context.configuration.address;
	rweb::serve(create_filter(application_context, room)).run(address).await;
}

pub fn create_filter(application_context: ApplicationContext, room: Room) -> BoxedFilter<(impl Reply,)> {
	websocket_filter(application_context, room.clone())
		.or(rweb::path("api").and(rest_api::rest_api(room)))
		.or(bundled_frontend_filter())
		.boxed()
}

fn websocket_filter(application_context: ApplicationContext, room: Room) -> BoxedFilter<(impl Reply,)> {
	rweb::path("ws")
		.and(rweb::ws())
		.map(
			move |ws: rweb::ws::Ws /*, room: Room, application_context: ApplicationContext*/| {
				let room = room.clone();
				let application_context = application_context.clone();

				ws.max_send_queue(1)
					.max_message_size(10 * 1024)
					.max_frame_size(10 * 1024)
					.on_upgrade(move |websocket| {
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
	use rust_embed::RustEmbed;
	use rweb::filters;
	use rweb::http::HeaderMap;
	use rweb::path::Tail;

	#[derive(RustEmbed)]
	#[folder = "$CARGO_MANIFEST_DIR/../communityvi-frontend/build"]
	struct FrontendBundle;

	let bundled_file_handler = BundledFileHandler::new::<FrontendBundle>();

	filters::path::tail()
		.and(filters::header::headers_cloned())
		.map(move |path: Tail, headers: HeaderMap| bundled_file_handler.request(path.as_str(), &headers))
		.boxed()
}

#[cfg(not(feature = "bundle-frontend"))]
fn bundled_frontend_filter() -> BoxedFilter<(Response<Body>,)> {
	rweb::any().and_then(|| ready(Err(rweb::reject::not_found()))).boxed()
}
