use crate::client::{Client, ClientId};
use crate::message::{ClientRequest, OrderedMessage, ServerResponse, WebSocketMessage};
use crate::room::Room;
use futures::future::join;
use futures::StreamExt;
use futures::{FutureExt, SinkExt, Stream};
use log::{debug, error, info, warn};
use std::convert::TryFrom;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use warp::filters::ws::Ws;
use warp::reply::with_header;
use warp::{Filter, Rejection, Reply};

const REFERENCE_CLIENT_HTML: &str = include_str!("../static/reference.html");

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
			let reply = ws.on_upgrade(move |websocket| async move {
				let (websocket_sink, websocket_stream) = websocket.split();
				let (message_sender, message_receiver) =
					futures::channel::mpsc::channel::<OrderedMessage<ServerResponse>>(1);
				let client_id = room.add_client(message_sender);
				let message_receive_future = message_receiver
					.map(|message| WebSocketMessage::from(&message))
					.map(Ok)
					.forward(websocket_sink.sink_map_err(|_| ()))
					.map(|_| ());

				let stream_future = receive_messages(websocket_stream, client_id, &room);

				// type erasure for faster compile times!
				let handle_messages_and_respond: Pin<Box<dyn Future<Output = ()> + Send>> =
					Box::pin(join(message_receive_future, stream_future).map(|_| ()));
				handle_messages_and_respond.await;

				room.remove_client(client_id);
			});
			Box::pin(async { Ok(Box::new(reply) as Box<dyn Reply>) })
				as Pin<Box<dyn Future<Output = Result<Box<dyn Reply>, Rejection>> + Send>> // type erasure for faster compile times!
		})
		.boxed();

	let reference_client_filter = warp::get()
		.and(warp::path("reference"))
		.and(warp::path::end())
		.map(|| with_header(REFERENCE_CLIENT_HTML, "Content-Type", "text/html; charset=utf-8"));

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

async fn receive_messages(
	mut websocket_stream: impl Stream<Item = Result<warp::ws::Message, warp::Error>> + Unpin + Send,
	client_id: ClientId,
	room: &Room,
) {
	loop {
		let websocket_message = match websocket_stream.next().await {
			Some(Ok(message)) => message,
			Some(Err(error)) => {
				error!("Error streaming websocket messages: {}", error);
				return; // stop reading from websocket when it is broken
			}
			None => return, // websocket has been closed
		};

		let OrderedMessage { number, message } = match OrderedMessage::<ClientRequest>::try_from(websocket_message) {
			Ok(message) => message,
			Err(error) => {
				error!("Error converting messages: {}", error);
				continue; // single message wasn't deserializable, continue with next one
			}
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
		handle_message(room, &client, message).await
	}
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => room.singlecast(&client, ServerResponse::Pong).await,
		ClientRequest::Chat { message } => room.broadcast(ServerResponse::Chat { message }).await,
		ClientRequest::Pong => info!("Received Pong from client: {}", client.id()),
		ClientRequest::Register { .. } => unreachable!("Register messages are handled in 'register_client'."),
	}
}
