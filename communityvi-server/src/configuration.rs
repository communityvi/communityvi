use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Configuration {
	pub address: String,
	pub port: u16,
}

impl TryFrom<&str> for Configuration {
	type Error = toml::de::Error;

	fn try_from(text: &str) -> Result<Self, Self::Error> {
		toml::from_str(text)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::fs::File;
	use std::io::Read;

	#[test]
	fn should_deserialize_configuration() {
		const TEST_FILE_PATH: &str = "test/files/test-configuration.toml";
		let mut file = File::open(TEST_FILE_PATH).expect("Failed to open test configuration file.");
		let mut text = String::new();
		file.read_to_string(&mut text)
			.expect("Failed to read test configuration file.");

		let Configuration { address, port } =
			Configuration::try_from(text.as_str()).expect("Failed to parse configuration.");

		assert_eq!("127.0.0.1", address);
		assert_eq!(8000, port);
	}
}
