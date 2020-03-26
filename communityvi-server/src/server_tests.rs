use crate::message::{ClientRequest, Message, ServerResponse};
use crate::server::create_server;
use futures::{FutureExt, SinkExt, StreamExt};
use http::Request;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::convert::TryFrom;
use tokio::runtime;
use tokio_tungstenite::tungstenite;
use url::Url;

const URL: &str = "ws://localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

async fn websocket_connection() -> (
	impl futures::Sink<Message<ClientRequest>, Error = ()>,
	impl futures::Stream<Item = Message<ServerResponse>>,
) {
	let request = Request::builder()
		.uri(format!("{}/ws", URL))
		.body(())
		.expect("Failed to build handshake request.");
	let (websocket_stream, _response) = tokio_tungstenite::connect_async(request)
		.await
		.map_err(|error| panic!("Websocket connection failed: {}", error))
		.unwrap();
	let (sink, stream) = websocket_stream.split();
	let stream = stream.map(|result| {
		let websocket_message = result.expect("Stream error.");
		let json = websocket_message.to_text().expect("No text message received.");
		Message::<ServerResponse>::try_from(json).expect("Failed to parse JSON response")
	});
	let sink = sink.sink_map_err(|error| panic!("{}", error)).with(|message| {
		let websocket_message =
			tungstenite::Message::text(serde_json::to_string(&message).expect("Failed to convert message to JSON"));
		futures::future::ok(websocket_message)
	});
	(sink, stream)
}

#[test]
fn should_respond_to_websocket_messages() {
	let future = async {
		let (mut sink, stream) = websocket_connection().await;
		let message = Message {
			number: 42,
			message: ClientRequest::Ping,
		};
		sink.send(message).await.expect("Failed to sink message.");
		stream.take(1).collect().await
	};
	let messages: Vec<_> = test_future_with_running_server(future);
	assert_eq!(messages.len(), 1);
	assert_eq!(
		messages[0],
		Message {
			number: 0,
			message: ServerResponse::Pong,
		}
	);
}

#[test]
fn should_broadcast_messages() {
	let future = async move {
		let message = r#"Hello everyone \o/"#;
		let request = Message {
			number: 1337,
			message: ClientRequest::Chat {
				message: message.to_string(),
			},
		};
		let (mut sink1, mut stream1) = websocket_connection().await;
		let (_sink2, mut stream2) = websocket_connection().await;

		let expected_response = Message {
			number: 0,
			message: ServerResponse::Chat {
				message: message.to_string(),
			},
		};

		sink1.send(request).await.expect("Failed to sink broadcast message.");

		assert_eq!(
			expected_response,
			stream1.next().await.expect("Failed to receive response on client 1")
		);
		assert_eq!(
			expected_response,
			stream2.next().await.expect("Failed to receive response on client 2")
		);
	};
	test_future_with_running_server(future);
}

#[test]
fn test_messages_should_have_sequence_numbers() {
	let future = async move {
		let first_request = Message {
			number: 0,
			message: ClientRequest::Chat {
				message: "first".into(),
			},
		};
		let second_request = Message {
			number: 1,
			message: ClientRequest::Chat {
				message: "second".into(),
			},
		};

		let expected_first_response = Message {
			number: 0,
			message: ServerResponse::Chat {
				message: "first".into(),
			},
		};
		let expected_second_response = Message {
			number: 1,
			message: ServerResponse::Chat {
				message: "second".into(),
			},
		};

		let (mut sink, mut stream) = websocket_connection().await;
		sink.send(first_request).await.expect("Failed to sink first message.");
		sink.send(second_request).await.expect("Failed to sink second message.");

		let first_response = stream.next().await.expect("Didn't receive first message");
		assert_eq!(expected_first_response, first_response);
		let second_response = stream.next().await.expect("Didn't receive second message");
		assert_eq!(expected_second_response, second_response);
	};
	test_future_with_running_server(future);
}

fn test_future_with_running_server<OutputType, FutureType>(future_to_test: FutureType) -> OutputType
where
	OutputType: Send + 'static,
	FutureType: std::future::Future<Output = OutputType> + Send + 'static,
{
	let _guard = TEST_MUTEX.lock();
	let mut runtime = runtime::Builder::new()
		.threaded_scheduler()
		.enable_all()
		.build()
		.expect("Failed to create runtime");
	let (sender, receiver) = futures::channel::oneshot::channel();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), receiver);
	let server_handle = runtime.spawn(server);

	let output = runtime.block_on(future_to_test);
	sender.send(()).expect("Failed to send shutdown.");
	runtime.block_on(server_handle).expect("Failed to join server.");
	output
}
