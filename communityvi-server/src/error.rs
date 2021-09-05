use crate::configuration::ConfigurationError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum CommunityviError {
	ConfigurationError(ConfigurationError),
	CommandlineError(String),
}

impl Display for CommunityviError {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		match self {
			CommunityviError::ConfigurationError(error) => write!(formatter, "Failed to load configuration: {}", error),
			CommunityviError::CommandlineError(message) => {
				write!(formatter, "Failed to parse commandline: {}", message)
			}
		}
	}
}

impl Error for CommunityviError {}

impl From<ConfigurationError> for CommunityviError {
	fn from(configuration_error: ConfigurationError) -> Self {
		CommunityviError::ConfigurationError(configuration_error)
	}
}

impl From<clap::Error> for CommunityviError {
	fn from(clap_error: clap::Error) -> Self {
		CommunityviError::CommandlineError(clap_error.to_string())
	}
}
