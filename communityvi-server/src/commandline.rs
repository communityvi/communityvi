use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::error::CommunityviError;
use crate::server::run_server;
use crate::utils::time_source::TimeSource;
use log::info;
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
		let time_source = TimeSource::default();
		let application_context = ApplicationContext::new(configuration, time_source);

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
				run_server(&application_context, false).await
			}
			BaseCommand::Demo => {
				info!(
					"Starting server in demo mode. Go to 'http://{}/reference' to access the demo.",
					application_context.configuration.address
				);
				run_server(&application_context, true).await
			}
			BaseCommand::Configuration => println!("{:?}", application_context.configuration),
		}
		Ok(())
	}
}
