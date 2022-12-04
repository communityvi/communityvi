#![allow(clippy::unused_async)]
use crate::connection::receiver::MessageReceiver;
use crate::connection::sender::MessageSender;
use crate::context::ApplicationContext;
use crate::lifecycle::run_client;
use crate::room::Room;
use crate::server::rest_api::{finish_openapi_specification, rest_api};
use crate::utils::websocket_message_conversion::{
	axum_websocket_message_to_tungstenite_message, tungstenite_message_to_axum_websocket_message,
};
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use axum::extract::{ws::WebSocket, Extension, State, WebSocketUpgrade};
use axum::Router;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use std::future::ready;
use std::sync::Arc;

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

pub fn create_router(application_context: ApplicationContext, room: Room) -> Router {
	let mut open_api = OpenApi::default();

	let router = ApiRouter::new()
		.api_route(
			"/ws",
			get_with(websocket_handler, |operation| {
				operation.summary("Start a websocket client session")
			}),
		)
		.nest_api_service("/api", rest_api().with_state(application_context.clone()))
		.finish_api_with(&mut open_api, finish_openapi_specification)
		.with_state(application_context)
		.layer(Extension(room))
		.layer(Extension(Arc::new(open_api)));

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
	State(application_context): State<ApplicationContext>,
) -> impl IntoApiResponse {
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
