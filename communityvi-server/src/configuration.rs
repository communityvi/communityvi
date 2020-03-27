use serde::Deserialize;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Configuration {
	#[serde(with = "socket_addr_deserializer")]
	pub address: SocketAddr,
	pub log_filters: String,
}

impl Configuration {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Configuration, ConfigurationError> {
		let text = read_to_string(path)?;

		Ok(Configuration::try_from(text.as_str())?)
	}
}

impl TryFrom<&str> for Configuration {
	type Error = toml::de::Error;

	fn try_from(text: &str) -> Result<Self, Self::Error> {
		toml::from_str(text)
	}
}

#[derive(Debug)]
pub enum ConfigurationError {
	DeserializationError(String),
	IoError(String),
}

impl Display for ConfigurationError {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		match self {
			ConfigurationError::DeserializationError(message) => {
				write!(formatter, "Failed to deserialize with error: {}", message)
			}
			ConfigurationError::IoError(message) => write!(formatter, "IO operation failed: {}", message),
		}
	}
}

impl From<std::io::Error> for ConfigurationError {
	fn from(io_error: std::io::Error) -> Self {
		ConfigurationError::IoError(io_error.to_string())
	}
}

impl From<toml::de::Error> for ConfigurationError {
	fn from(toml_error: toml::de::Error) -> Self {
		ConfigurationError::DeserializationError(toml_error.to_string())
	}
}

// See https://serde.rs/custom-date-format.html
mod socket_addr_deserializer {
	use serde::{self, Deserialize, Deserializer};
	use std::net::SocketAddr;
	use std::str::FromStr;

	pub fn deserialize<'deserializer, D>(deserializer: D) -> Result<SocketAddr, D::Error>
	where
		D: Deserializer<'deserializer>,
	{
		let string = String::deserialize(deserializer)?;
		SocketAddr::from_str(string.as_str()).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn should_deserialize_configuration() {
		const TEST_FILE_PATH: &str = "test/files/test-configuration.toml";

		let Configuration { address, log_filters } = Configuration::from_file(TEST_FILE_PATH).unwrap();

		assert_eq!(SocketAddr::from_str("127.0.0.1:8000").unwrap(), address);
		assert_eq!("info", log_filters);
	}
}
