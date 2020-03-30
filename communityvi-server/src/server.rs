use crate::client::{Client, ClientId};
use crate::message::{ClientRequest, OrderedMessage, ServerResponse, WebSocketMessage};
use crate::room::Room;
use futures::channel::mpsc;
use futures::future::join;
use futures::{FutureExt, SinkExt, Stream};
use futures::{StreamExt, TryStreamExt};
use log::{debug, error, info, warn};
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
			let reply = ws.on_upgrade(move |websocket| {
				async move {
					let (websocket_sink, websocket_stream) = websocket.split();
					let (message_sender, message_receiver) = mpsc::channel::<OrderedMessage<ServerResponse>>(2);

					let mut client_request_stream = websocket_stream_to_client_requests(websocket_stream);

					let client_id = register_client(&room, &mut client_request_stream, message_sender).await;

					// convert server responses into websocket messages and send them
					let message_receive_future = message_receiver
						.map(|message| WebSocketMessage::from(&message))
						.map(Ok)
						.forward(websocket_sink.sink_map_err(|_| ()))
						.map(|_| ());

					let stream_future = if let Some(client_id) = client_id {
						receive_messages(client_request_stream, client_id, &room).boxed()
					} else {
						async {}.boxed()
					};

					// type erasure for faster compile times!
					let handle_messages_and_respond: Pin<Box<dyn Future<Output = ()> + Send>> =
						Box::pin(join(message_receive_future, stream_future).map(|_| ()));
					handle_messages_and_respond.await;

					client_id.map(|client_id| room.remove_client(client_id));
				}
				.boxed()
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

async fn register_client(
	room: &Room,
	request_stream: &mut (impl Stream<Item = OrderedMessage<ClientRequest>> + Send + Unpin),
	mut response_sender: mpsc::Sender<OrderedMessage<ServerResponse>>,
) -> Option<ClientId> {
	let request = match request_stream.next().await {
		None => {
			error!("Client registration failed. Socket closed prematurely.");
			return None;
		}
		Some(request) => request,
	};

	let (number, name) = if let OrderedMessage {
		number,
		message: ClientRequest::Register { name },
	} = request
	{
		(number, name)
	} else {
		error!("Client registration failed. Invalid request: {:?}", request);

		let invalid_message_response = OrderedMessage {
			number: 0,
			message: ServerResponse::InvalidMessage,
		};
		let _ = response_sender.send(invalid_message_response).await;
		return None;
	};

	if number != 0 {
		error!(
			"Client registration failed. Invalid message number: {}, should be 0.",
			number
		);
		let invalid_message_response = OrderedMessage {
			number: 0,
			message: ServerResponse::InvalidMessage,
		};
		let _ = response_sender.send(invalid_message_response).await;
		return None;
	}

	let client_handle = room.add_client(name, response_sender);
	let hello_response = ServerResponse::Hello { id: client_handle.id() };
	if room.singlecast(&client_handle, hello_response).await.is_ok() {
		let name = client_handle.name().to_string();
		let id = client_handle.id();

		// Drop the client_handle so that the lock on the concurrent hashmap is released for the broadcast
		std::mem::drop(client_handle);

		info!("Registered client: {} {}", id, name);

		room.broadcast(ServerResponse::Joined { id, name }).await;

		Some(id)
	} else {
		None
	}
}

fn websocket_stream_to_client_requests(
	websocket_stream: impl Stream<Item = Result<warp::ws::Message, warp::Error>> + Unpin + Send,
) -> impl Stream<Item = OrderedMessage<ClientRequest>> + Send + Unpin {
	websocket_stream
		.inspect_err(|error| {
			error!("Error streaming websocket message: {}, result.", error);
		})
		.take_while(|result| futures::future::ready(result.is_ok()))
		.map(|result| match result {
			Ok(message) => message,
			Err(_) => unreachable!("Error's can't happen, they have been filtered out."),
		})
		.map(OrderedMessage::from)
}

async fn receive_messages(
	mut client_request_stream: impl Stream<Item = OrderedMessage<ClientRequest>> + Send + Unpin,
	client_id: ClientId,
	room: &Room,
) {
	loop {
		let OrderedMessage { number, message } = match client_request_stream.next().await {
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
			let _ = room.singlecast(&client, ServerResponse::InvalidMessage).await;
		}
		ClientRequest::Invalid { .. } => {
			let _ = room.singlecast(&client, ServerResponse::InvalidMessage).await;
		}
		ClientRequest::Close => {
			info!("Close message received for Client: {} {}", client.id(), client.name());
			let _ = room.singlecast(&client, ServerResponse::Bye).await;
		}
	}
}
