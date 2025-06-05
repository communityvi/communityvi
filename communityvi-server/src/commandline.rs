use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::error::CommunityviError;
use crate::server::run_server;
use crate::utils::time_source::TimeSource;
use log::info;

#[derive(clap::Parser)]
pub struct Commandline {
	#[clap(short = 'c', long = "config-file", default_value = "configuration.toml")]
	pub configuration_file_path: String,
	#[clap(subcommand)]
	pub command: Option<BaseCommand>,
}

#[derive(clap::Parser)]
pub enum BaseCommand {
	/// Run the communityvi server (websocket mode only)
	Run,
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
		let time_source = TimeSource::default();
		let application_context = ApplicationContext::new(configuration, time_source)
			.await
			.expect("Failed to create application context.");

		env_logger::Builder::new()
			.parse_filters(&application_context.configuration.log_filters)
			.init();

		let base_command = self.command.unwrap_or_default();
		match base_command {
			BaseCommand::Run => {
				info!(
					"Starting server. Start websocket connections at 'ws://{}/ws'.",
					application_context.configuration.address
				);
				run_server(application_context).await?;
			}
			BaseCommand::Configuration => println!("{:?}", application_context.configuration),
		}
		Ok(())
	}
}
