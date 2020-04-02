use crate::client::{Client, ClientId};
use crate::connection::{register_client, ServerConnection};
use crate::message::{ClientRequest, ErrorResponse, OrderedMessage, ServerResponse};
use crate::room::Room;
use futures::FutureExt;
use log::{debug, error, info, warn};
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
			let reply = ws.on_upgrade(move |websocket| {
				async move {
					if let Some((client_id, server_connection)) = register_client(&room, websocket).await {
						handle_messages(server_connection, client_id, &room).await;
						room.remove_client(client_id);
					}
				}
				.boxed()
			});
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
			// prevent XSS
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

async fn handle_messages(mut server_connection: ServerConnection, client_id: ClientId, room: &Room) {
	loop {
		let OrderedMessage { number, message } = match server_connection.receive().await {
			Some(message) => message,
			None => return,
		};
		debug!(
			"Received {:?} message {} from {}",
			std::mem::discriminant(&message),
			number,
			client_id
		);

		let client = match room.get_client_by_id(client_id) {
			Some(client_handle) => client_handle,
			None => {
				warn!("Couldn't find Client: {}", client_id);
				return;
			}
		};
		handle_message(room, &client, message).await;
	}
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => {
			let _ = room.singlecast(&client, ServerResponse::Pong).await;
		}
		ClientRequest::Chat { message } => {
			room.broadcast(ServerResponse::Chat {
				sender_id: client.id(),
				sender_name: client.name().to_string(),
				message,
			})
			.await
		}
		ClientRequest::Pong => info!("Received Pong from client: {}", client.id()),
		ClientRequest::Register { .. } => {
			error!(
				"Client: {} tried to register even though it is already registered.",
				client.id()
			);
			let _ = room
				.singlecast(
					&client,
					ServerResponse::Error {
						error: ErrorResponse::InvalidOperation,
					},
				)
				.await;
		}
	}
}
