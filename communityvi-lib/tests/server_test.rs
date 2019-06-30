use communityvi_lib::server::create_server;
use futures::future::Future;
use std::fmt::Debug;
use tokio::runtime::Runtime;

const URL: &str = "http://localhost:8000";

#[test]
fn should_get_offset() {
	let client = reqwest::r#async::Client::new();
	let get_request = client.get(URL).build().unwrap();

	let future = client.execute(get_request).and_then(|mut response| response.json());

	let offset: u64 = test_future_with_running_server(future);
	assert_eq!(offset, 42);
}

#[test]
fn should_set_offset() {
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
