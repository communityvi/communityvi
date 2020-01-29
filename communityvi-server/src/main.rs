use communityvi_lib::server::create_server;
use futures::FutureExt;
use tokio::task::JoinError;

#[tokio::main]
async fn main() -> Result<(), JoinError> {
	let (_sender, receiver) = futures::channel::oneshot::channel::<()>();
	let receiver = receiver.then(|_| futures::future::ready(()));
	let server = create_server(([127, 0, 0, 1], 8000), receiver);

	tokio::spawn(server).await
}
