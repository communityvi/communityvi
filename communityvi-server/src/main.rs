use communityvi_lib::server::create_server;
use futures::FutureExt;

fn main() {
	let (_sender, receiver) = futures::channel::oneshot::channel::<()>();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), receiver);

	tokio_compat::run_std(server);
}
