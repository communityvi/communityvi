use crate::message::server_response::PlaybackStateResponse;
use crate::message::{MessageError, WebSocketMessage};
use crate::room::client_id::ClientId;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Broadcast {
	ClientJoined(ClientJoinedBroadcast),
	ClientLeft(ClientLeftBroadcast),
	Chat(ChatBroadcast),
	MediumInserted(MediumInsertedBroadcast),
	PlaybackStateChanged(PlaybackStateChangedBroadcast),
}

macro_rules! broadcast_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for Broadcast {
			fn from(broadcast: $struct_type) -> Broadcast {
				Broadcast::$enum_case(broadcast)
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
}

broadcast_from_struct!(Chat, ChatBroadcast);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MediumInsertedBroadcast {
	pub inserted_by_name: String,
	pub inserted_by_id: ClientId,
	pub name: String,
	pub length_in_milliseconds: u64,
}

broadcast_from_struct!(MediumInserted, MediumInsertedBroadcast);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlaybackStateChangedBroadcast {
	pub changed_by_name: String,
	pub changed_by_id: ClientId,
	pub skipped: bool,
	pub playback_state: PlaybackStateResponse,
}

broadcast_from_struct!(PlaybackStateChanged, PlaybackStateChangedBroadcast);

impl TryFrom<&WebSocketMessage> for Broadcast {
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

impl From<&Broadcast> for WebSocketMessage {
	fn from(message: &Broadcast) -> Self {
		let json = serde_json::to_string(message).expect("Failed to serialize broadcast message to JSON.");
		WebSocketMessage::text(json)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn chat_broadcast_should_serialize_and_deserialize() {
		let chat_broadcast = Broadcast::Chat(ChatBroadcast {
			sender_id: ClientId::from(42),
			sender_name: "Hedwig".to_string(),
			message: "hello".to_string(),
		});
		let json = serde_json::to_string(&chat_broadcast).expect("Failed to serialize Chat broadcast to JSON");
		assert_eq!(
			r#"{"type":"chat","sender_id":42,"sender_name":"Hedwig","message":"hello"}"#,
			json
		);

		let deserialized_chat_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize Chat broadcast from JSON");
		assert_eq!(chat_broadcast, deserialized_chat_broadcast);
	}

	#[test]
	fn client_joined_broadcast_should_serialize_and_deserialize() {
		let joined_broadcast = Broadcast::ClientJoined(ClientJoinedBroadcast {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json =
			serde_json::to_string(&joined_broadcast).expect("Failed to serialize ClientJoined broadcast to JSON");
		assert_eq!(r#"{"type":"client_joined","id":42,"name":"Hedwig"}"#, json);

		let deserialized_joined_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize ClientJoined broadcast from JSON");
		assert_eq!(joined_broadcast, deserialized_joined_broadcast);
	}

	#[test]
	fn client_left_broadcast_should_serialize_and_deserialize() {
		let client_left_broadcast = Broadcast::ClientLeft(ClientLeftBroadcast {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json =
			serde_json::to_string(&client_left_broadcast).expect("Failed to serialize ClientLeft broadcast to JSON");
		assert_eq!(r#"{"type":"client_left","id":42,"name":"Hedwig"}"#, json);

		let deserialized_client_left_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize ClientLeft broadcast from JSON");
		assert_eq!(client_left_broadcast, deserialized_client_left_broadcast);
	}

	#[test]
	fn medium_inserted_broadcast_should_serialize_and_deserialize() {
		let medium_inserted_broadcast = Broadcast::MediumInserted(MediumInsertedBroadcast {
			inserted_by_name: "Squirrel".to_string(),
			inserted_by_id: ClientId::from(42),
			name: "The Acorn".to_string(),
			length_in_milliseconds: 20 * 60 * 1000,
		});
		let json = serde_json::to_string(&medium_inserted_broadcast)
			.expect("Failed to serialize MediumInserted broadcast to JSON");
		assert_eq!(
			r#"{"type":"medium_inserted","inserted_by_name":"Squirrel","inserted_by_id":42,"name":"The Acorn","length_in_milliseconds":1200000}"#,
			json
		);

		let deserialized_medium_inserted_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize MediumInserted broadcast from JSON");
		assert_eq!(medium_inserted_broadcast, deserialized_medium_inserted_broadcast);
	}

	#[test]
	fn playback_state_changed_broadcast_for_playing_should_serialize_and_deserialize() {
		let playback_state_changed_broadcast = Broadcast::PlaybackStateChanged(PlaybackStateChangedBroadcast {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Playing {
				start_time_in_milliseconds: -1337,
			},
		});
		let json = serde_json::to_string(&playback_state_changed_broadcast)
			.expect("Failed to serialize PlaybackStateChanged broadcast to JSON");
		assert_eq!(
			r#"{"type":"playback_state_changed","changed_by_name":"Alice","changed_by_id":0,"skipped":false,"playback_state":{"type":"playing","start_time_in_milliseconds":-1337}}"#,
			json
		);

		let deserialized_playback_state_changed_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize PlaybackStateChanged broadcast from JSON");
		assert_eq!(
			playback_state_changed_broadcast,
			deserialized_playback_state_changed_broadcast
		);
	}

	#[test]
	fn playback_state_changed_broadcast_for_paused_should_serialize_and_deserialize() {
		let playback_state_changed_broadcast = Broadcast::PlaybackStateChanged(PlaybackStateChangedBroadcast {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Paused {
				position_in_milliseconds: 42,
			},
		});
		let json = serde_json::to_string(&playback_state_changed_broadcast)
			.expect("Failed to serialize PlaybackStateChanged broadcast to JSON");
		assert_eq!(
			r#"{"type":"playback_state_changed","changed_by_name":"Alice","changed_by_id":0,"skipped":false,"playback_state":{"type":"paused","position_in_milliseconds":42}}"#,
			json
		);

		let deserialized_playback_state_changed_broadcast: Broadcast =
			serde_json::from_str(&json).expect("Failed to deserialize PlaybackStateChanged broadcast from JSON");
		assert_eq!(
			playback_state_changed_broadcast,
			deserialized_playback_state_changed_broadcast
		);
	}
}
