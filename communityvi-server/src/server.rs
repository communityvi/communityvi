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
					let (message_sender, message_receiver) = mpsc::channel::<OrderedMessage<ServerResponse>>(1);

					let mut client_request_stream = websocket_stream_to_client_requests(websocket_stream);

					let client_id = if let Some(client_id) =
						register_client(&room, &mut client_request_stream, message_sender).await
					{
						client_id
					} else {
						return;
					};

					// convert server responses into websocket messages and send them
					let message_receive_future = message_receiver
						.map(|message| WebSocketMessage::from(&message))
						.map(Ok)
						.forward(websocket_sink.sink_map_err(|_| ()))
						.map(|_| ());

					let stream_future = receive_messages(client_request_stream, client_id, &room);

					// type erasure for faster compile times!
					let handle_messages_and_respond: Pin<Box<dyn Future<Output = ()> + Send>> =
						Box::pin(join(message_receive_future, stream_future).map(|_| ()));
					handle_messages_and_respond.await;

					room.remove_client(client_id);
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
	response_sender: mpsc::Sender<OrderedMessage<ServerResponse>>,
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
		//FIXME: Send error message to client
		return None;
	};

	if number != 0 {
		unimplemented!("Should fail here.");
	}

	room.add_client(name, response_sender).await
}

fn websocket_stream_to_client_requests(
	websocket_stream: impl Stream<Item = Result<warp::ws::Message, warp::Error>> + Unpin + Send,
) -> impl Stream<Item = OrderedMessage<ClientRequest>> + Send + Unpin {
	websocket_stream
		.inspect_err(|error| {
			error!("Error streaming websocket message: {}, result.", error);
		})
		.take_while(|result| {
			let is_ok = result.is_ok();
			futures::future::ready(is_ok)
		})
		.map(|result| match result {
			Ok(message) => OrderedMessage::<ClientRequest>::from(message),
			Err(_) => unreachable!("Error's can't happen, they have been filtered out."),
		})
}

async fn receive_messages(
	client_request_stream: impl Stream<Item = OrderedMessage<ClientRequest>> + Send,
	client_id: ClientId,
	room: &Room,
) {
	client_request_stream
		.for_each(|ordered_message| async {
			let OrderedMessage { number, message } = ordered_message;
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
		})
		.await
}

async fn handle_message(room: &Room, client: &Client, request: ClientRequest) {
	match request {
		ClientRequest::Ping => room.singlecast(&client, ServerResponse::Pong).await,
		ClientRequest::Chat { message } => room.broadcast(ServerResponse::Chat { message }).await,
		ClientRequest::Pong => info!("Received Pong from client: {}", client.id()),
		ClientRequest::Register { .. } => unreachable!("Register messages are handled in 'register_client'."),
		ClientRequest::Invalid { .. } => unimplemented!(),
	}
}
