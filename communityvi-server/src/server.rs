#![allow(clippy::unused_async)]
use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::rest_api::rest_api;
use crate::utils::websocket_message_conversion::{
	axum_websocket_message_to_tungstenite_message, tungstenite_message_to_axum_websocket_message,
};
use axum::extract::{ws::WebSocket, Extension, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use std::future::ready;

mod file_bundle;
mod rest_api;

pub async fn run_server(application_context: ApplicationContext) {
	let room = Room::new(
		application_context.reference_timer.clone(),
		application_context.configuration.room_size_limit,
	);
	let address = application_context.configuration.address;
	axum::Server::bind(&address)
		.serve(create_router(application_context, room).into_make_service())
		.await
		.unwrap();
}

pub fn create_router(application_context: ApplicationContext, room: Room) -> Router<()> {
	let reference_timer = application_context.reference_timer.clone();
	let router = Router::new()
		.route("/ws", get(websocket_handler))
		.nest("/api", rest_api(reference_timer))
		.layer(Extension(room))
		.layer(Extension(application_context));

	#[cfg(feature = "bundle-frontend")]
	{
		#[derive(rust_embed::RustEmbed)]
		#[folder = "$CARGO_MANIFEST_DIR/../communityvi-frontend/build"]
		struct FrontendBundle;

		let bundled_frontend_handler = file_bundle::BundledFileHandlerBuilder::default()
			.with_rust_embed::<FrontendBundle>()
			.build();
		router.fallback_service(axum::routing::get_service(bundled_frontend_handler))
	}

	#[cfg(not(feature = "bundle-frontend"))]
	{
		router
	}
}

async fn websocket_handler(
	websocket: WebSocketUpgrade,
	Extension(room): Extension<Room>,
	Extension(application_context): Extension<ApplicationContext>,
) -> impl IntoResponse {
	websocket
		.max_send_queue(1)
		.max_message_size(10 * 1024)
		.max_frame_size(10 * 1024)
		.on_upgrade(move |websocket| run_websocket_connection(websocket, room, application_context))
}

async fn run_websocket_connection(websocket: WebSocket, room: Room, application_context: ApplicationContext) {
	let (sink, stream) = websocket.split();

	let message_sender =
		MessageSender::from(sink.with(|message| ready(tungstenite_message_to_axum_websocket_message(message))));
	let message_receiver = MessageReceiver::new(
		stream
			.map_ok(axum_websocket_message_to_tungstenite_message)
			.map_err(Into::into),
		message_sender.clone(),
	);

	run_client(application_context, room, message_sender, message_receiver).await;
}
