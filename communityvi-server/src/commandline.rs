use crate::configuration::Configuration;
use crate::context::ApplicationContext;
use crate::error::CommunityviError;
use crate::server::run_server;
use crate::utils::time_source::TimeSource;
use std::io::{IsTerminal, stdout};
use tracing::info;

#[derive(clap::Parser)]
pub struct Commandline {
	#[clap(short = 'c', long = "config-file", default_value = "configuration.toml")]
	pub configuration_file_path: String,
	#[clap(subcommand)]
	pub command: Option<BaseCommand>,
}

#[derive(clap::Parser, Default)]
pub enum BaseCommand {
	/// Run the communityvi server (websocket mode only)
	#[default]
	Run,
	/// Print the configuration
	Configuration,
}

impl Commandline {
	pub async fn run(self) -> Result<(), CommunityviError> {
		let configuration = Configuration::from_file(&self.configuration_file_path)?;
		let time_source = TimeSource::default();
		let application_context = ApplicationContext::new(configuration, time_source)
			.await
			.expect("Failed to create application context.");

		tracing_subscriber::fmt()
			.with_env_filter(&application_context.configuration.log_filters)
			.with_ansi(stdout().is_terminal())
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
