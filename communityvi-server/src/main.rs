use communityvi_lib::server::create_server;

fn main() {
	let (_sender, receiver) = futures::sync::oneshot::channel();
	let server = create_server(([127, 0, 0, 1], 8000), receiver);
	tokio::run(server);
}
