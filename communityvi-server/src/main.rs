use crate::configuration::Configuration;
use crate::error::CommunityviError;
use crate::server::create_server;
use futures::FutureExt;

mod atomic_sequence;
mod client;
mod client_id_sequence;
mod configuration;
mod error;
mod message;
mod room;
mod server;
#[cfg(test)]
mod server_tests;

#[tokio::main]
async fn main() -> Result<(), CommunityviError> {
	const CONFIGURATION_FILE_PATH: &str = "configuration.toml";
	let configuration = Configuration::from_file(CONFIGURATION_FILE_PATH)?;

	env_logger::Builder::new()
		.parse_filters(&configuration.log_filters)
		.init();

	let (_shutdown_sender, shutdown_receiver) = futures::channel::oneshot::channel::<()>();
	let shutdown_handle = shutdown_receiver.then(|_| futures::future::ready(()));
	let server = create_server(configuration.address, shutdown_handle, true);

	server.await;
	Ok(())
}
