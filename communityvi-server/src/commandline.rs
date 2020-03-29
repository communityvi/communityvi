use crate::configuration::Configuration;
use crate::error::CommunityviError;
use crate::server::create_server;
use futures::FutureExt;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Commandline {
	#[structopt(short = "c", long = "config-file", default_value = "configuration.toml")]
	pub configuration_file_path: String,
	#[structopt(subcommand)]
	pub command: Option<BaseCommand>,
}

#[derive(StructOpt)]
pub enum BaseCommand {
	/// Run the communityvi server (websocket mode only)
	Run,
	/// Run the communityvi server with demo client on `/reference`
	Demo,
	/// Print the configuration
	Configuration,
}

impl Default for BaseCommand {
	fn default() -> Self {
		Self::Run
	}
}

impl Commandline {
	pub async fn run(self) -> Result<(), CommunityviError> {
		let configuration = Configuration::from_file(&self.configuration_file_path)?;

		env_logger::Builder::new()
			.parse_filters(&configuration.log_filters)
			.init();

		let (_shutdown_sender, shutdown_receiver) = futures::channel::oneshot::channel::<()>();
		let shutdown_handle = shutdown_receiver.then(|_| futures::future::ready(()));

		let base_command = self.command.unwrap_or_default();
		match base_command {
			BaseCommand::Run => create_server(configuration.address, shutdown_handle, false).await,
			BaseCommand::Demo => create_server(configuration.address, shutdown_handle, true).await,
			BaseCommand::Configuration => println!("{:?}", configuration),
		}
		Ok(())
	}
}
