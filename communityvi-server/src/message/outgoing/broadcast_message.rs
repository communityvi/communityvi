use crate::message::outgoing::success_message::PlaybackStateResponse;
use crate::message::{MessageError, WebSocketMessage};
use crate::room::client_id::ClientId;
use crate::room::medium::{Medium, VersionedMedium};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum BroadcastMessage {
	ClientJoined(ClientJoinedBroadcast),
	ClientLeft(ClientLeftBroadcast),
	Chat(ChatBroadcast),
	MediumStateChanged(MediumStateChangedBroadcast),
}

macro_rules! broadcast_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for BroadcastMessage {
			fn from(broadcast: $struct_type) -> BroadcastMessage {
				BroadcastMessage::$enum_case(broadcast)
			}
		}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClientJoinedBroadcast {
	pub id: ClientId,
	pub name: String,
}

broadcast_from_struct!(ClientJoined, ClientJoinedBroadcast);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClientLeftBroadcast {
	pub id: ClientId,
	pub name: String,
}

broadcast_from_struct!(ClientLeft, ClientLeftBroadcast);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChatBroadcast {
	pub sender_id: ClientId,
	pub sender_name: String,
	pub message: String,
	pub counter: u64,
}

broadcast_from_struct!(Chat, ChatBroadcast);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MediumStateChangedBroadcast {
	pub changed_by_name: String,
	pub changed_by_id: ClientId,
	pub medium: VersionedMediumBroadcast,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct VersionedMediumBroadcast {
	pub version: u64,
	#[serde(flatten)]
	pub medium: MediumBroadcast,
}

impl VersionedMediumBroadcast {
	pub fn new(versioned_medium: VersionedMedium, skipped: bool) -> Self {
		Self {
			medium: MediumBroadcast::new(versioned_medium.medium, skipped),
			version: versioned_medium.version,
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum MediumBroadcast {
	FixedLength {
		name: String,
		length_in_milliseconds: u64,
		playback_skipped: bool,
		playback_state: PlaybackStateResponse,
	},
	Empty,
}

impl MediumBroadcast {
	pub fn new(medium: impl Into<Medium>, skipped: bool) -> Self {
		match medium.into() {
			Medium::FixedLength(medium) => MediumBroadcast::FixedLength {
				name: medium.name,
				length_in_milliseconds: medium.length.num_milliseconds() as u64,
				playback_skipped: skipped,
				playback_state: medium.playback.into(),
			},
			Medium::Empty => MediumBroadcast::Empty,
		}
	}
}

broadcast_from_struct!(MediumStateChanged, MediumStateChangedBroadcast);

impl TryFrom<&WebSocketMessage> for BroadcastMessage {
	type Error = MessageError;

	fn try_from(websocket_message: &WebSocketMessage) -> Result<Self, Self::Error> {
		match websocket_message {
			WebSocketMessage::Text(json) => {
				serde_json::from_str(json).map_err(|error| MessageError::DeserializationFailed {
					error: error.to_string(),
					json: json.to_string(),
				})
			}
			_ => Err(MessageError::WrongMessageType(websocket_message.clone())),
		}
	}
}

impl From<&BroadcastMessage> for WebSocketMessage {
	fn from(message: &BroadcastMessage) -> Self {
		let json = serde_json::to_string(message).expect("Failed to serialize broadcast message to JSON.");
		WebSocketMessage::text(json)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn chat_broadcast_should_serialize_and_deserialize() {
		let chat_broadcast = BroadcastMessage::Chat(ChatBroadcast {
			sender_id: ClientId::from(42),
			sender_name: "Hedwig".to_string(),
			message: "hello".to_string(),
			counter: 1337,
		});
		let json = serde_json::to_string(&chat_broadcast).expect("Failed to serialize Chat broadcast to JSON");
		assert_eq!(
			r#"{"type":"chat","sender_id":42,"sender_name":"Hedwig","message":"hello","counter":1337}"#,
			json
		);

		let deserialized_chat_broadcast: BroadcastMessage =
			serde_json::from_str(&json).expect("Failed to deserialize Chat broadcast from JSON");
		assert_eq!(chat_broadcast, deserialized_chat_broadcast);
	}

	#[test]
	fn client_joined_broadcast_should_serialize_and_deserialize() {
		let joined_broadcast = BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json =
			serde_json::to_string(&joined_broadcast).expect("Failed to serialize ClientJoined broadcast to JSON");
		assert_eq!(r#"{"type":"client_joined","id":42,"name":"Hedwig"}"#, json);

		let deserialized_joined_broadcast: BroadcastMessage =
			serde_json::from_str(&json).expect("Failed to deserialize ClientJoined broadcast from JSON");
		assert_eq!(joined_broadcast, deserialized_joined_broadcast);
	}

	#[test]
	fn client_left_broadcast_should_serialize_and_deserialize() {
		let client_left_broadcast = BroadcastMessage::ClientLeft(ClientLeftBroadcast {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json =
			serde_json::to_string(&client_left_broadcast).expect("Failed to serialize ClientLeft broadcast to JSON");
		assert_eq!(r#"{"type":"client_left","id":42,"name":"Hedwig"}"#, json);

		let deserialized_client_left_broadcast: BroadcastMessage =
			serde_json::from_str(&json).expect("Failed to deserialize ClientLeft broadcast from JSON");
		assert_eq!(client_left_broadcast, deserialized_client_left_broadcast);
	}

	#[test]
	fn medium_state_changed_broadcast_for_paused_should_serialize_and_deserialize() {
		let medium_state_changed_broadcast = BroadcastMessage::MediumStateChanged(MediumStateChangedBroadcast {
			changed_by_name: "Squirrel".to_string(),
			changed_by_id: ClientId::from(42),
			medium: VersionedMediumBroadcast {
				medium: MediumBroadcast::FixedLength {
					name: "The Acorn".to_string(),
					length_in_milliseconds: 20 * 60 * 1000,
					playback_skipped: false,
					playback_state: PlaybackStateResponse::Paused {
						position_in_milliseconds: 0,
					},
				},
				version: 0,
			},
		});
		let json = serde_json::to_string_pretty(&medium_state_changed_broadcast)
			.expect("Failed to serialize MediumStateChanged broadcast to JSON");
		assert_eq!(
			r#"{
  "type": "medium_state_changed",
  "changed_by_name": "Squirrel",
  "changed_by_id": 42,
  "medium": {
    "version": 0,
    "type": "fixed_length",
    "name": "The Acorn",
    "length_in_milliseconds": 1200000,
    "playback_skipped": false,
    "playback_state": {
      "type": "paused",
      "position_in_milliseconds": 0
    }
  }
}"#,
			json
		);

		let deserialized_medium_state_changed_broadcast: BroadcastMessage =
			serde_json::from_str(&json).expect("Failed to deserialize MediumInserted broadcast from JSON");
		assert_eq!(
			medium_state_changed_broadcast,
			deserialized_medium_state_changed_broadcast
		);
	}

	#[test]
	fn medium_state_changed_broadcast_for_playing_should_serialize_and_deserialize() {
		let medium_state_changed_broadcast = BroadcastMessage::MediumStateChanged(MediumStateChangedBroadcast {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			medium: VersionedMediumBroadcast {
				medium: MediumBroadcast::FixedLength {
					name: "Metropolis".to_string(),
					length_in_milliseconds: 153 * 60 * 1000,
					playback_skipped: false,
					playback_state: PlaybackStateResponse::Playing {
						start_time_in_milliseconds: -1337,
					},
				},
				version: 0,
			},
		});
		let json = serde_json::to_string_pretty(&medium_state_changed_broadcast)
			.expect("Failed to serialize PlaybackStateChanged broadcast to JSON");
		assert_eq!(
			r#"{
  "type": "medium_state_changed",
  "changed_by_name": "Alice",
  "changed_by_id": 0,
  "medium": {
    "version": 0,
    "type": "fixed_length",
    "name": "Metropolis",
    "length_in_milliseconds": 9180000,
    "playback_skipped": false,
    "playback_state": {
      "type": "playing",
      "start_time_in_milliseconds": -1337
    }
  }
}"#,
			json
		);

		let deserialized_medium_state_changed_broadcast: BroadcastMessage =
			serde_json::from_str(&json).expect("Failed to deserialize MediumStateChanged broadcast from JSON");
		assert_eq!(
			medium_state_changed_broadcast,
			deserialized_medium_state_changed_broadcast
		);
	}
}
