use serde::{Deserialize, Serialize};

use crate::message::{
	deserialize_message_from_str, serialize_message_to_websocket_message, Message, MessageError, WebSocketMessage,
};
use crate::room::client_id::ClientId;
use crate::room::state::medium::playback_state::PlaybackState;
use crate::room::state::medium::{Medium, SomeMedium};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ServerResponse {
	Chat(ChatResponse),
	Hello(HelloResponse),
	Joined(JoinedResponse),
	ReferenceTime(ReferenceTimeResponse),
	MediumInserted(MediumInsertedResponse),
	PlaybackStateChanged(PlaybackStateChangedResponse),
	Left(LeftResponse),
	Error(ErrorResponse),
}

macro_rules! server_response_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for ServerResponse {
			fn from(response: $struct_type) -> ServerResponse {
				ServerResponse::$enum_case(response)
			}
		}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChatResponse {
	pub sender_id: ClientId,
	pub sender_name: String,
	pub message: String,
}

server_response_from_struct!(Chat, ChatResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HelloResponse {
	pub id: ClientId,
	pub current_medium: Option<MediumResponse>,
}

server_response_from_struct!(Hello, HelloResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct JoinedResponse {
	pub id: ClientId,
	pub name: String,
}

server_response_from_struct!(Joined, JoinedResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ReferenceTimeResponse {
	pub milliseconds: u64,
}

server_response_from_struct!(ReferenceTime, ReferenceTimeResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MediumInsertedResponse {
	pub inserted_by_name: String,
	pub inserted_by_id: ClientId,
	pub name: String,
	pub length_in_milliseconds: u64,
}

server_response_from_struct!(MediumInserted, MediumInsertedResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlaybackStateChangedResponse {
	pub changed_by_name: String,
	pub changed_by_id: ClientId,
	pub skipped: bool,
	pub playback_state: PlaybackStateResponse,
}

server_response_from_struct!(PlaybackStateChanged, PlaybackStateChangedResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LeftResponse {
	pub id: ClientId,
	pub name: String,
}

server_response_from_struct!(Left, LeftResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ErrorResponse {
	pub error: ErrorResponseType,
	pub message: String,
}

server_response_from_struct!(Error, ErrorResponse);

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorResponseType {
	InvalidFormat,
	InvalidOperation,
	NoMedium,
	InternalServerError,
}

impl Message for ServerResponse {}

impl From<&ServerResponse> for WebSocketMessage {
	fn from(response: &ServerResponse) -> Self {
		serialize_message_to_websocket_message(response)
	}
}

impl TryFrom<&str> for ServerResponse {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, MessageError> {
		deserialize_message_from_str(json)
	}
}

impl TryFrom<&WebSocketMessage> for ServerResponse {
	type Error = MessageError;

	fn try_from(websocket_message: &WebSocketMessage) -> Result<Self, MessageError> {
		match websocket_message {
			WebSocketMessage::Text(json) => json.as_str().try_into(),
			_ => Err(MessageError::WrongMessageType(websocket_message.clone())),
		}
	}
}
#[cfg(test)]
mod test {
	use super::*;
	use chrono::Duration;

	#[test]
	fn chat_response_should_serialize_and_deserialize() {
		let chat_response = ServerResponse::Chat(ChatResponse {
			sender_id: ClientId::from(42),
			sender_name: "Hedwig".to_string(),
			message: "hello".to_string(),
		});
		let json = serde_json::to_string(&chat_response).expect("Failed to serialize Chat response to JSON");
		assert_eq!(
			r#"{"type":"chat","sender_id":42,"sender_name":"Hedwig","message":"hello"}"#,
			json
		);

		let deserialized_chat_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Chat response from JSON");
		assert_eq!(chat_response, deserialized_chat_response);
	}

	#[test]
	fn joined_response_should_serialize_and_deserialize() {
		let joined_response = ServerResponse::Joined(JoinedResponse {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json = serde_json::to_string(&joined_response).expect("Failed to serialize Joined response to JSON");
		assert_eq!(r#"{"type":"joined","id":42,"name":"Hedwig"}"#, json);

		let deserialized_joined_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Joined response from JSON");
		assert_eq!(joined_response, deserialized_joined_response);
	}

	#[test]
	fn left_response_should_serialize_and_deserialize() {
		let left_response = ServerResponse::Left(LeftResponse {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		});
		let json = serde_json::to_string(&left_response).expect("Failed to serialize Left response to JSON");
		assert_eq!(r#"{"type":"left","id":42,"name":"Hedwig"}"#, json);

		let deserialized_left_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Left response from JSON");
		assert_eq!(left_response, deserialized_left_response);
	}

	#[test]
	fn hello_response_without_medium_should_serialize_and_deserialize() {
		let hello_response = ServerResponse::Hello(HelloResponse {
			id: 42.into(),
			current_medium: None,
		});
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(r#"{"type":"hello","id":42,"current_medium":null}"#, json);

		let deserialized_hello_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn hello_response_with_medium_should_serialize_and_deserialize() {
		let hello_response = ServerResponse::Hello(HelloResponse {
			id: 42.into(),
			current_medium: Some(MediumResponse::FixedLength {
				name: "WarGames".to_string(),
				length_in_milliseconds: Duration::minutes(114).num_milliseconds() as u64,
				playback_state: PlaybackStateResponse::Paused {
					position_in_milliseconds: 0,
				},
			}),
		});
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(
			r#"{"type":"hello","id":42,"current_medium":{"type":"fixed_length","name":"WarGames","length_in_milliseconds":6840000,"playback_state":{"type":"paused","position_in_milliseconds":0}}}"#,
			json
		);

		let deserialized_hello_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn reference_time_response_should_serialize_and_deserialize() {
		let reference_time_response = ServerResponse::ReferenceTime(ReferenceTimeResponse { milliseconds: 1337 });
		let json = serde_json::to_string(&reference_time_response)
			.expect("Failed to serialize ReferenceTime response to JSON");
		assert_eq!(r#"{"type":"reference_time","milliseconds":1337}"#, json);

		let deserialized_reference_time_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize ReferenceTime response from JSON");
		assert_eq!(reference_time_response, deserialized_reference_time_response);
	}

	#[test]
	fn medium_inserted_response_should_serialize_and_deserialize() {
		let medium_inserted_response = ServerResponse::MediumInserted(MediumInsertedResponse {
			inserted_by_name: "Squirrel".to_string(),
			inserted_by_id: ClientId::from(42),
			name: "The Acorn".to_string(),
			length_in_milliseconds: 20 * 60 * 1000,
		});
		let json = serde_json::to_string(&medium_inserted_response)
			.expect("Failed to serialize MediumInserted response to JSON");
		assert_eq!(
			r#"{"type":"medium_inserted","inserted_by_name":"Squirrel","inserted_by_id":42,"name":"The Acorn","length_in_milliseconds":1200000}"#,
			json
		);

		let deserialized_medium_inserted_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize MediumInserted response from JSON");
		assert_eq!(medium_inserted_response, deserialized_medium_inserted_response);
	}

	#[test]
	fn playback_state_changed_response_for_playing_should_serialize_and_deserialize() {
		let playback_state_changed_response = ServerResponse::PlaybackStateChanged(PlaybackStateChangedResponse {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Playing {
				start_time_in_milliseconds: -1337,
			},
		});
		let json = serde_json::to_string(&playback_state_changed_response)
			.expect("Failed to serialize PlaybackStateChanged response to JSON");
		assert_eq!(
			r#"{"type":"playback_state_changed","changed_by_name":"Alice","changed_by_id":0,"skipped":false,"playback_state":{"type":"playing","start_time_in_milliseconds":-1337}}"#,
			json
		);

		let deserialized_playback_state_changed_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize PlaybackStateChanged response from JSON");
		assert_eq!(
			playback_state_changed_response,
			deserialized_playback_state_changed_response
		);
	}

	#[test]
	fn playback_state_changed_response_for_paused_should_serialize_and_deserialize() {
		let playback_state_changed_response = ServerResponse::PlaybackStateChanged(PlaybackStateChangedResponse {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Paused {
				position_in_milliseconds: 42,
			},
		});
		let json = serde_json::to_string(&playback_state_changed_response)
			.expect("Failed to serialize PlaybackStateChanged response to JSON");
		assert_eq!(
			r#"{"type":"playback_state_changed","changed_by_name":"Alice","changed_by_id":0,"skipped":false,"playback_state":{"type":"paused","position_in_milliseconds":42}}"#,
			json
		);

		let deserialized_playback_state_changed_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize PlaybackStateChanged response from JSON");
		assert_eq!(
			playback_state_changed_response,
			deserialized_playback_state_changed_response
		);
	}

	#[test]
	fn invalid_format_error_response_should_serialize_and_deserialize() {
		let invalid_format_error_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InvalidFormat,
			message: "�".to_string(),
		});
		let json = serde_json::to_string(&invalid_format_error_response)
			.expect("Failed to serialize InvalidFormat error response to JSON");
		assert_eq!(r#"{"type":"error","error":"invalid_format","message":"�"}"#, json);

		let deserialized_invalid_format_error_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize InvalidFormat error response from JSON");
		assert_eq!(
			invalid_format_error_response,
			deserialized_invalid_format_error_response
		);
	}

	#[test]
	fn invalid_operation_error_response_should_serialize_and_deserialize() {
		let invalid_operation_error_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InvalidOperation,
			message: "I'm a teapot.".to_string(),
		});
		let json = serde_json::to_string(&invalid_operation_error_response)
			.expect("Failed to serialize InvalidOperation error response to JSON");
		assert_eq!(
			r#"{"type":"error","error":"invalid_operation","message":"I'm a teapot."}"#,
			json
		);

		let deserialized_invalid_operation_error_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize InvalidOperation error response from JSON");
		assert_eq!(
			invalid_operation_error_response,
			deserialized_invalid_operation_error_response
		);
	}

	#[test]
	fn internal_server_error_response_should_serialize_and_deserialize() {
		let internal_server_error_error_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InternalServerError,
			message: "I've found a bug crawling around my circuits.".to_string(),
		});
		let json = serde_json::to_string(&internal_server_error_error_response)
			.expect("Failed to serialize InternalServerError error response to JSON");
		assert_eq!(
			r#"{"type":"error","error":"internal_server_error","message":"I've found a bug crawling around my circuits."}"#,
			json
		);

		let deserialized_internal_server_error_error_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize InternalServerError error response from JSON");
		assert_eq!(
			internal_server_error_error_response,
			deserialized_internal_server_error_error_response
		);
	}
}
