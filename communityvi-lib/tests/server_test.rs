use communityvi_lib::message::{Message, OrderedMessage, TextMessage};
use communityvi_lib::server::create_server;
use futures::FutureExt;
use futures01::future::join_all;
use futures01::future::Future;
use futures01::sink::Sink;
use futures01::stream::Stream;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::time::Duration;
use tokio_compat::runtime::Runtime;
use url::Url;

const URL: &str = "http://localhost:8000";
lazy_static! {
	static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn should_set_and_get_offset() {
	let client = reqwest10::Client::new();
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

	let offset: u64 = test_std_future_with_running_server(future);
	assert_eq!(offset, new_offset);
}

fn websocket_connection() -> impl Future<
	Item = (
		impl Sink<SinkItem = Message, SinkError = ()>,
		impl Stream<Item = OrderedMessage, Error = ()>,
	),
	Error = (),
> {
	let mut websocket_url = Url::parse(&format!("{}/ws", URL)).expect("Failed to parse URL");
	websocket_url.set_scheme("ws").expect("Failed to set URL scheme.");
	let request = tungstenite::handshake::client::Request {
		url: websocket_url,
		extra_headers: None,
	};
	tokio_tungstenite::connect_async(request)
		.map_err(|error| panic!("Websocket connection failed: {}", error))
		.map(|(websocket_stream, _response)| {
			let (sink, stream) = websocket_stream.split();
			let stream = stream
				.map_err(|error| panic!("Stream error: {}", error))
				.map(|websocket_message| {
					let json = websocket_message.to_text().expect("No text message received.");
					OrderedMessage::try_from(json).expect("Failed to parse JSON response")
				});
			let sink = sink.sink_map_err(|error| panic!("{}", error)).with(|message: Message| {
				let websocket_message = tungstenite::Message::text(
					serde_json::to_string(&message).expect("Failed to convert message to JSON"),
				);
				futures01::future::ok(websocket_message)
			});
			(sink, stream)
		})
}

#[test]
fn should_respond_to_websocket_messages() {
	let future = websocket_connection().and_then(|(sink, stream)| {
		let message = Message::Ping(TextMessage {
			text: "Hello World!".into(),
		});
		let send_future = sink.send(message).map(|_| ());
		let receive_future = stream.take(1).collect().map(|messages| {
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
		});
		let futures: Vec<Box<dyn Future<Item = (), Error = ()> + Send>> =
			vec![Box::new(send_future), Box::new(receive_future)];
		join_all(futures)
	});
	test_future_with_running_server(future);
}

#[test]
fn should_broadcast_messages() {
	let future = websocket_connection()
		.and_then(|(sink1, stream1)| websocket_connection().map(|(_sink2, stream2)| (sink1, stream1, stream2)))
		.and_then(|(sink, stream1, stream2)| {
			let message = Message::Chat(TextMessage {
				text: r#"Hello everyone \o/"#.into(),
			});
			let ordered_message1 = OrderedMessage {
				number: 0,
				message: message.clone(),
			};
			let ordered_message2 = ordered_message1.clone();

			let send_future = sink.send(message).map(|_| ());
			let receive_future1 = stream1.take(1).collect().map(move |messages| {
				assert_eq!(messages.len(), 1);
				assert_eq!(messages[0], ordered_message1);
			});
			let receive_future2 = stream2.take(1).collect().map(move |messages| {
				assert_eq!(messages.len(), 1);
				assert_eq!(messages[0], ordered_message2);
			});
			let futures: Vec<Box<dyn Future<Item = (), Error = ()> + Send + Sync>> = vec![
				Box::new(send_future),
				Box::new(receive_future1),
				Box::new(receive_future2),
			];
			join_all(futures).map(|_| ()).map_err(|_| ())
		});
	test_future_with_running_server(future);
}

#[test]
fn test_messages_should_have_sequence_numbers() {
	let future = websocket_connection().and_then(|(sink, stream)| {
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

		let send_future = sink
			.send(first_message)
			.and_then(move |sink| sink.send(second_message))
			.map(|_sink| ());

		let receive_future = stream.take(2).collect().map(move |ordered_messages| {
			assert_eq!(ordered_messages.len(), 2);
			assert_eq!(ordered_messages[0], first_ordered_message);
			assert_eq!(ordered_messages[1], second_ordered_message);
		});

		let futures: Vec<Box<dyn Future<Item = (), Error = ()> + Send + Sync>> =
			vec![Box::new(send_future), Box::new(receive_future)];
		join_all(futures).map(|_results| ())
	});
	test_future_with_running_server(future);
}

fn test_future_with_running_server<ItemType, ErrorType, FutureType>(future_to_test: FutureType) -> ItemType
where
	ItemType: Send + 'static,
	ErrorType: Send + Debug + 'static,
	FutureType: Future<Item = ItemType, Error = ErrorType> + Send + 'static,
{
	let guard = TEST_MUTEX.lock();
	let (sender, receiver) = futures::channel::oneshot::channel();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), receiver);
	let mut runtime = Runtime::new().expect("Failed to create runtime");
	runtime.spawn_std(server);

	let future = future_to_test.then(|test_result| {
		sender.send(()).expect("Must send shutdown signal.");
		test_result
	});

	let result = runtime.block_on(future);
	std::thread::sleep(Duration::from_millis(20)); // Wait for port to be free to use again
	std::mem::drop(guard);
	match result {
		Err(error) => panic!("{:?}", error),
		Ok(value) => value,
	}
}

fn test_std_future_with_running_server<OutputType, FutureType>(future_to_test: FutureType) -> OutputType
where
	OutputType: Send + 'static,
	FutureType: std::future::Future<Output = OutputType> + Send + 'static,
{
	let _guard = TEST_MUTEX.lock();
	let (sender, receiver) = futures::channel::oneshot::channel();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), receiver);
	let mut runtime = Runtime::new().expect("Failed to create runtime");
	runtime.spawn_std(server);

	let future = async {
		let output = future_to_test.await;
		sender.send(()).expect("Failed to send shutdown");
		output
	};

	let output = runtime.block_on_std(future);
	std::thread::sleep(Duration::from_millis(20)); // Wait for port to be free to use again
	output
}
