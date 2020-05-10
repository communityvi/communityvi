use serde::{Deserialize, Serialize};

use crate::room::client::Client;
use crate::room::client_id::ClientId;
use crate::room::state::medium::playback_state::PlaybackState;
use crate::room::state::medium::{Medium, SomeMedium};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SuccessMessage {
	Hello {
		id: ClientId,
		clients: Vec<ClientResponse>,
		current_medium: Option<MediumResponse>,
	},
	ReferenceTime {
		milliseconds: u64,
	},
	Success,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClientResponse {
	pub id: ClientId,
	pub name: String,
}

impl From<Client> for ClientResponse {
	fn from(client: Client) -> Self {
		Self {
			id: client.id(),
			name: client.name().to_string(),
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MediumResponse {
	FixedLength {
		name: String,
		length_in_milliseconds: u64,
		playback_state: PlaybackStateResponse,
	},
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum PlaybackStateResponse {
	Playing { start_time_in_milliseconds: i64 },
	Paused { position_in_milliseconds: u64 },
}

impl From<PlaybackState> for PlaybackStateResponse {
	fn from(playback_state: PlaybackState) -> Self {
		match playback_state {
			PlaybackState::Playing { start_time } => Self::Playing {
				start_time_in_milliseconds: start_time.num_milliseconds(),
			},
			PlaybackState::Paused { at_position } => Self::Paused {
				position_in_milliseconds: at_position.num_milliseconds() as u64,
			},
		}
	}
}

impl From<&SomeMedium> for MediumResponse {
	fn from(some_medium: &SomeMedium) -> Self {
		match some_medium {
			SomeMedium::FixedLength(fixed_length) => Self::FixedLength {
				name: fixed_length.name().to_string(),
				length_in_milliseconds: fixed_length.length.num_milliseconds() as u64,
				playback_state: fixed_length.playback_state().into(),
			},
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use chrono::Duration;

	#[test]
	fn hello_response_without_medium_should_serialize_and_deserialize() {
		let hello_response = SuccessMessage::Hello {
			id: 42.into(),
			clients: vec![],
			current_medium: None,
		};
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(r#"{"type":"hello","id":42,"clients":[],"current_medium":null}"#, json);

		let deserialized_hello_response: SuccessMessage =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn hello_response_with_medium_should_serialize_and_deserialize() {
		let hello_response = SuccessMessage::Hello {
			id: 42.into(),
			clients: vec![ClientResponse {
				id: ClientId::from(8080),
				name: "IMSAI".to_string(),
			}],
			current_medium: Some(MediumResponse::FixedLength {
				name: "WarGames".to_string(),
				length_in_milliseconds: Duration::minutes(114).num_milliseconds() as u64,
				playback_state: PlaybackStateResponse::Paused {
					position_in_milliseconds: 0,
				},
			}),
		};
		let json = serde_json::to_string_pretty(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(
			r#"{
  "type": "hello",
  "id": 42,
  "clients": [
    {
      "id": 8080,
      "name": "IMSAI"
    }
  ],
  "current_medium": {
    "type": "fixed_length",
    "name": "WarGames",
    "length_in_milliseconds": 6840000,
    "playback_state": {
      "type": "paused",
      "position_in_milliseconds": 0
    }
  }
}"#,
			json
		);

		let deserialized_hello_response: SuccessMessage =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn reference_time_response_should_serialize_and_deserialize() {
		let reference_time_response = SuccessMessage::ReferenceTime { milliseconds: 1337 };
		let json = serde_json::to_string(&reference_time_response)
			.expect("Failed to serialize ReferenceTime response to JSON");
		assert_eq!(r#"{"type":"reference_time","milliseconds":1337}"#, json);

		let deserialized_reference_time_response: SuccessMessage =
			serde_json::from_str(&json).expect("Failed to deserialize ReferenceTime response from JSON");
		assert_eq!(reference_time_response, deserialized_reference_time_response);
	}

	#[test]
	fn success_response_should_serialize_and_deserialize() {
		let success_response = SuccessMessage::Success;
		let json = serde_json::to_string(&success_response).expect("Failed to serialize Success response to JSON");
		assert_eq!(r#"{"type":"success"}"#, json);

		let deserialized_success_response: SuccessMessage =
			serde_json::from_str(&json).expect("Failed to deserialize Success response from JSON");
		assert_eq!(success_response, deserialized_success_response);
	}
}
