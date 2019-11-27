use communityvi_lib::server::create_server;
use futures::{FutureExt, TryFutureExt};

fn main() {
	let (_sender, receiver) = futures::channel::oneshot::channel::<()>();
	let server = create_server(([127, 0, 0, 1], 8000), receiver.boxed().compat());

	tokio_compat::run(server);
}
