use communityvi_lib::server::create_server;
use futures::future::Future;
use futures::stream::Stream;
use tokio::runtime::Runtime;

#[test]
fn should_get_offset() {
	let (sender, receiver) = futures::sync::oneshot::channel();
	let future = create_server(([127, 0, 0, 1], 8000), receiver);
	let mut runtime = Runtime::new().unwrap();
	runtime.spawn(future);

	let client = reqwest::r#async::Client::new();
	let get_request = client.get("http://localhost:8000").build().unwrap();

	let future = client
		.execute(get_request)
		.and_then(|response| response.into_body().concat2().map(|bytes| bytes.to_vec()))
		.map(|bytes| String::from_utf8(bytes).unwrap())
		.map(|string| {
			let number: u64 = string.parse().unwrap();
			number
		})
		.map_err(|error| panic!("{:?}", error))
		.map(|value| {
			sender.send(()).unwrap();
			value
		});
	let offset = runtime.block_on(future).unwrap();
	assert_eq!(offset, 42);
}
