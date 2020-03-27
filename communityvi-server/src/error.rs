use crate::configuration::ConfigurationError;
use serde::export::Formatter;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum CommunityviError {
	ConfigurationError(ConfigurationError),
}

impl Display for CommunityviError {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		match self {
			CommunityviError::ConfigurationError(error) => write!(formatter, "Failed to load configuration: {}", error),
		}
	}
}

impl Error for CommunityviError {}

impl From<ConfigurationError> for CommunityviError {
	fn from(configuration_error: ConfigurationError) -> Self {
		CommunityviError::ConfigurationError(configuration_error)
	}
}
