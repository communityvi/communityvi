use crate::message::{Message, OrderedMessage, TextMessage};
use crate::server::create_server;
use futures::{FutureExt, SinkExt, StreamExt};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::convert::TryFrom;
use tokio::runtime;
use tokio_tungstenite::tungstenite;
use url::Url;

const URL: &str = "http://localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn should_set_and_get_offset() {
	let client = reqwest::Client::new();
	let new_offset = 1337u64;
	let future = async move {
		let set_offset_request = client
			.post(&format!("{url}/{offset}", url = URL, offset = new_offset))
			.build()
			.unwrap();
		let set_offset_was_successful = client
			.execute(set_offset_request)
			.await
			.expect("Error during post request.")
			.status()
			.is_success();
		assert!(set_offset_was_successful);

		let get_offset_request = client.get(URL).build().unwrap();
		client
			.execute(get_offset_request)
			.await
			.expect("Error during get request.")
			.json()
			.await
			.expect("Failed to decode response.")
	};

	let offset: u64 = test_future_with_running_server(future);
	assert_eq!(offset, new_offset);
}

async fn websocket_connection() -> (
	impl futures::Sink<Message, Error = ()>,
	impl futures::Stream<Item = OrderedMessage>,
) {
	let mut websocket_url = Url::parse(&format!("{}/ws", URL)).expect("Failed to parse URL");
	websocket_url.set_scheme("ws").expect("Failed to set URL scheme.");
	let request = tungstenite::handshake::client::Request {
		url: websocket_url,
		extra_headers: None,
	};
	let (websocket_stream, _response) = tokio_tungstenite::connect_async(request)
		.await
		.map_err(|error| panic!("Websocket connection failed: {}", error))
		.unwrap();
	let (sink, stream) = websocket_stream.split();
	let stream = stream.map(|result| {
		let websocket_message = result.expect("Stream error.");
		let json = websocket_message.to_text().expect("No text message received.");
		OrderedMessage::try_from(json).expect("Failed to parse JSON response")
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
		let message = Message::Ping(TextMessage {
			text: "Hello World!".into(),
		});
		sink.send(message).await.expect("Failed to sink message.");
		stream.take(1).collect().await
	};
	let messages: Vec<_> = test_future_with_running_server(future);
	assert_eq!(messages.len(), 1);
	assert_eq!(
		messages[0],
		OrderedMessage {
			number: 0,
			message: Message::Pong(TextMessage {
				text: "Hello World!".into()
			})
		}
	);
}

#[test]
fn should_broadcast_messages() {
	let message = Message::Chat(TextMessage {
		text: r#"Hello everyone \o/"#.into(),
	});
	let ordered_message = OrderedMessage {
		number: 0,
		message: message.clone(),
	};
	let message_for_future = message.clone();
	let future = async move {
		let (mut sink1, stream1) = websocket_connection().await;
		let (_sink2, stream2) = websocket_connection().await;

		sink1
			.send(message_for_future)
			.await
			.expect("Failed to sink broadcast message.");

		let received_messages1: Vec<_> = stream1.take(1).collect().await;
		let received_messages2: Vec<_> = stream2.take(1).collect().await;
		(received_messages1, received_messages2)
	};
	let (received_messages1, received_messages2) = test_future_with_running_server(future);
	assert_eq!(received_messages1.len(), 1);
	assert_eq!(received_messages2.len(), 1);
	assert_eq!(received_messages1[0], ordered_message);
	assert_eq!(received_messages2[0], ordered_message);
}

#[test]
fn test_messages_should_have_sequence_numbers() {
	let first_message = Message::Chat(TextMessage { text: "first".into() });
	let second_message = Message::Chat(TextMessage { text: "second".into() });
	let first_ordered_message = OrderedMessage {
		number: 0,
		message: first_message.clone(),
	};
	let second_ordered_message = OrderedMessage {
		number: 1,
		message: second_message.clone(),
	};

	let first_message_for_future = first_message.clone();
	let second_message_for_future = second_message.clone();
	let future = async move {
		let (mut sink, stream) = websocket_connection().await;

		let first_message = first_message_for_future;
		let second_message = second_message_for_future.clone();

		sink.send(first_message).await.expect("Failed to sink first message.");
		sink.send(second_message).await.expect("Failed to sink second message.");

		let received_messages: Vec<_> = stream.take(2).collect().await;
		received_messages
	};
	let received_messages = test_future_with_running_server(future);
	assert_eq!(received_messages.len(), 2);
	assert_eq!(received_messages[0], first_ordered_message);
	assert_eq!(received_messages[1], second_ordered_message);
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
