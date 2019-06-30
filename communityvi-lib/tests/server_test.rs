use communityvi_lib::server::create_server;
use futures::future::Future;
use futures::stream::Stream;
use std::fmt::Debug;
use tokio::runtime::Runtime;

#[test]
fn should_get_offset() {
	let client = reqwest::r#async::Client::new();
	let get_request = client.get("http://localhost:8000").build().unwrap();

	let future = client
		.execute(get_request)
		.and_then(|response| response.into_body().concat2().map(|bytes| bytes.to_vec()))
		.map(|bytes| String::from_utf8(bytes).unwrap())
		.map(|string| {
			let number: u64 = string.parse().unwrap();
			number
		});

	let offset = test_future_with_running_server(future);
	assert_eq!(offset, 42);
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
