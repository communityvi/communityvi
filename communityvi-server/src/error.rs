use crate::configuration::ConfigurationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommunityviError {
	#[error("Failed to load configuration: {0}")]
	Configuration(#[from] ConfigurationError),
	#[error("Failed to parse commandline: {0}")]
	Commandline(#[from] clap::Error),
	#[error("IO error while serving requests: {0}")]
	Server(#[from] std::io::Error),
}
