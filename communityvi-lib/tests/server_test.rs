use communityvi_lib::message::{Message, TextMessage};
use communityvi_lib::server::create_server;
use futures::future::join_all;
use futures::future::Future;
use futures::sink::Sink;
use futures::stream::Stream;
use std::convert::TryFrom;
use std::fmt::Debug;
use tokio::runtime::Runtime;
use url::Url;

const URL: &str = "http://localhost:8000";

#[test]
fn should_set_and_get_offset() {
	let client = reqwest::r#async::Client::new();
	let new_offset = 1337u64;
	let post_request = client
		.post(&format!("{url}/{offset}", url = URL, offset = new_offset))
		.build()
		.unwrap();
	let get_request = client.get(URL).build().unwrap();

	let post_future = client.execute(post_request);
	let get_future = client.execute(get_request).and_then(|mut response| response.json());

	let future = post_future.and_then(|_| get_future);

	let offset: u64 = test_future_with_running_server(future);
	assert_eq!(offset, new_offset);
}

#[test]
fn should_respond_to_websocket_messages() {
	let mut websocket_url = Url::parse(&format!("{}/ws", URL)).expect("Failed to parse URL");
	websocket_url.set_scheme("ws").expect("Failed to set URL scheme.");
	let request = tungstenite::handshake::client::Request {
		url: websocket_url,
		extra_headers: None,
	};
	let future = tokio_tungstenite::connect_async(request)
		.map_err(|error| panic!("{}", error))
		.and_then(|(web_socket_stream, _response)| {
			let (sink, stream) = web_socket_stream.split();
			let message = Message::Ping(TextMessage {
				text: "Hello World!".into(),
			});
			let websocket_message =
				tungstenite::Message::text(serde_json::to_string(&message).expect("Failed to convert message to JSON"));
			let send_future = sink
				.sink_map_err(|error| panic!("{}", error))
				.send(websocket_message)
				.map(|_| ());
			let receive_future = stream
				.map(|websocket_message| {
					let json = websocket_message.to_text().expect("No text message received.");
					Message::try_from(json).expect("Failed to parse JSON response")
				})
				.take(1)
				.collect()
				.map(|messages| {
					assert_eq!(messages.len(), 1);
					assert_eq!(
						messages[0],
						Message::Pong(TextMessage {
							text: "Hello World!".into()
						})
					);
				})
				.map_err(|error| panic!("{}", error));
			let futures: Vec<Box<dyn Future<Item = (), Error = ()> + Send>> =
				vec![Box::new(send_future), Box::new(receive_future)];
			join_all(futures)
		});
	test_future_with_running_server(future);
}

fn test_future_with_running_server<ItemType, ErrorType, FutureType>(future_to_test: FutureType) -> ItemType
where
	ItemType: Send + 'static,
	ErrorType: Send + Debug + 'static,
	FutureType: Future<Item = ItemType, Error = ErrorType> + Send + 'static,
{
	let (sender, receiver) = futures::sync::oneshot::channel();
	let server = create_server(([127, 0, 0, 1], 8000), receiver);
	let mut runtime = Runtime::new().expect("Failed to create runtime");
	runtime.spawn(server);

	let future = future_to_test.then(|test_result| {
		sender.send(()).expect("Must send shutdown signal.");
		test_result
	});

	let result = runtime.block_on(future);
	match result {
		Err(error) => panic!("{:?}", error),
		Ok(value) => value,
	}
}
