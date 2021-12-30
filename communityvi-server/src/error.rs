use crate::configuration::ConfigurationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommunityviError {
	#[error("Failed to load configuration: {0}")]
	ConfigurationError(#[from] ConfigurationError),
	#[error("Failed to parse commandline: {0}")]
	CommandlineError(#[from] clap::Error),
}
