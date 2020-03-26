use crate::server::create_server;
use futures::FutureExt;

mod client;
mod message;
mod room;
mod server;
#[cfg(test)]
mod server_tests;
mod state;

#[tokio::main]
async fn main() {
	env_logger::init();
	let (_shutdown_sender, shutdown_receiver) = futures::channel::oneshot::channel::<()>();
	let shutdown_handle = shutdown_receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), shutdown_handle);

	server.await;
}
