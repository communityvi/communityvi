use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::Deserialize;
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Configuration {
	#[serde(with = "socket_addr_deserializer")]
	pub address: SocketAddr,
	pub log_filters: String,
	pub room_size_limit: usize,
	#[serde(with = "humantime_serde")]
	pub heartbeat_interval: std::time::Duration,
	pub missed_heartbeat_limit: u8,
	jwt_secret: Option<String>,
}

impl Configuration {
	#[cfg(test)]
	pub fn test() -> Self {
		Self {
			address: "127.0.0.1:8000".parse().unwrap(),
			log_filters: String::new(),
			room_size_limit: 10,
			heartbeat_interval: std::time::Duration::from_secs(2),
			missed_heartbeat_limit: 3,
			jwt_secret: None,
		}
	}

	pub fn from_file(path: impl AsRef<Path>) -> Result<Configuration, ConfigurationError> {
		let text = read_to_string(path)?;

		Ok(Configuration::try_from(text.as_str())?)
	}

	pub fn jwt_keys(&self) -> Result<Option<(EncodingKey, DecodingKey)>, jsonwebtoken::errors::Error> {
		if let Some(jwt_secret) = self.jwt_secret.clone() {
			let encoding_key = EncodingKey::from_base64_secret(&jwt_secret)?;
			let decoding_key = DecodingKey::from_base64_secret(&jwt_secret)?;
			Ok(Some((encoding_key, decoding_key)))
		} else {
			Ok(None)
		}
	}
}

impl TryFrom<&str> for Configuration {
	type Error = toml::de::Error;

	fn try_from(text: &str) -> Result<Self, Self::Error> {
		toml::from_str(text)
	}
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
	#[error("Failed to deserialize with error: {0}")]
	DeserializationError(#[from] toml::de::Error),
	#[error("IO operation failed: {0}")]
	IoError(#[from] std::io::Error),
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
	use base64::prelude::BASE64_STANDARD;
	use base64::Engine;
	use std::str::FromStr;

	const TEST_FILE_PATH: &str = "test/files/test-configuration.toml";

	#[test]
	fn should_deserialize_configuration() {
		let Configuration {
			address,
			log_filters,
			room_size_limit,
			heartbeat_interval,
			missed_heartbeat_limit,
			jwt_secret,
		} = Configuration::from_file(TEST_FILE_PATH).unwrap();

		assert_eq!(SocketAddr::from_str("127.0.0.1:8000").unwrap(), address);
		assert_eq!("info", log_filters);
		assert_eq!(42, room_size_limit);
		assert_eq!(std::time::Duration::from_secs(2), heartbeat_interval);
		assert_eq!(3, missed_heartbeat_limit);
		assert_eq!(Some(BASE64_STANDARD.encode("Hallo, Welt!")), jwt_secret);
	}

	#[test]
	fn should_construct_jwt_keys_from_secret() {
		let configuration = Configuration::from_file(TEST_FILE_PATH).unwrap();
		assert!(matches!(configuration.jwt_keys(), Ok(Some(_))));
	}
}
