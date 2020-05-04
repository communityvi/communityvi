use crate::room::client_id::ClientId;
use crate::room::state::medium::playback_state::PlaybackState;
use crate::room::state::medium::{Medium, SomeMedium};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub trait Message: Clone + Debug + DeserializeOwned + Serialize + PartialEq {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ClientRequest {
	Register {
		name: String,
	},
	Chat {
		message: String,
	},
	GetReferenceTime,
	InsertMedium {
		name: String,
		length_in_milliseconds: u64,
	},
	Play {
		skipped: bool,
		start_time_in_milliseconds: i64,
	},
	Pause {
		skipped: bool,
		position_in_milliseconds: u64,
	},
}

impl Message for ClientRequest {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ServerResponse {
	Chat {
		sender_id: ClientId,
		sender_name: String,
		message: String,
	},
	Hello {
		id: ClientId,
		current_medium: Option<MediumResponse>,
	},
	Joined {
		id: ClientId,
		name: String,
	},
	ReferenceTime {
		milliseconds: u64,
	},
	MediumInserted {
		inserted_by_name: String,
		inserted_by_id: ClientId,
		name: String,
		length_in_milliseconds: u64,
	},
	PlaybackStateChanged {
		changed_by_name: String,
		changed_by_id: ClientId,
		skipped: bool,
		playback_state: PlaybackStateResponse,
	},
	Left {
		id: ClientId,
		name: String,
	},
	Error {
		error: ErrorResponse,
		message: String,
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
pub enum ErrorResponse {
	InvalidFormat,
	InvalidOperation,
	NoMedium,
	InternalServerError,
}

impl Message for ServerResponse {}

pub type WebSocketMessage = tokio_tungstenite::tungstenite::Message;

impl From<&ClientRequest> for WebSocketMessage {
	fn from(request: &ClientRequest) -> Self {
		serialize_message_to_websocket_message(request)
	}
}

impl From<&ServerResponse> for WebSocketMessage {
	fn from(response: &ServerResponse) -> Self {
		serialize_message_to_websocket_message(response)
	}
}

fn serialize_message_to_websocket_message<MessageType: Message>(message: &MessageType) -> WebSocketMessage {
	let json = serde_json::to_string(message).expect("Failed to serialize message to JSON.");
	WebSocketMessage::text(json)
}

#[derive(Debug)]
pub enum MessageError {
	DeserializationFailed { error: String, json: String },
	WrongMessageType(WebSocketMessage),
}

impl Display for MessageError {
	fn fmt(&self, formatter: &mut Formatter) -> Result<(), std::fmt::Error> {
		match self {
			MessageError::DeserializationFailed { error, json } => write!(
				formatter,
				"Failed to deserialize message with error: '{}'; Message was '{}'",
				error, json
			),
			MessageError::WrongMessageType(message) => write!(
				formatter,
				"Wrong websocket message type. Expected text, got: {:?}",
				message
			),
		}
	}
}

impl Error for MessageError {}

impl TryFrom<&str> for ClientRequest {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, Self::Error> {
		deserialize_message_from_str(json)
	}
}

impl TryFrom<&str> for ServerResponse {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, MessageError> {
		deserialize_message_from_str(json)
	}
}

fn deserialize_message_from_str<MessageType: Message>(json: &str) -> Result<MessageType, MessageError> {
	serde_json::from_str(json).map_err(|error| MessageError::DeserializationFailed {
		error: error.to_string(),
		json: json.to_string(),
	})
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

impl TryFrom<&WebSocketMessage> for ClientRequest {
	type Error = MessageError;

	fn try_from(websocket_message: &WebSocketMessage) -> Result<Self, Self::Error> {
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
	fn chat_request_should_serialize_and_deserialize() {
		let chat_request = ClientRequest::Chat {
			message: "hello".into(),
		};
		let json = serde_json::to_string(&chat_request).expect("Failed to serialize Chat request to JSON");
		assert_eq!(r#"{"type":"chat","message":"hello"}"#, json);

		let deserialized_chat_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Chat request from JSON");
		assert_eq!(chat_request, deserialized_chat_request);
	}

	#[test]
	fn register_request_should_serialize_and_deserialize() {
		let register_request = ClientRequest::Register {
			name: "Ferris".to_string(),
		};
		let json = serde_json::to_string(&register_request).expect("Failed to serialize Register request to JSON");
		assert_eq!(r#"{"type":"register","name":"Ferris"}"#, json);

		let deserialized_register_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Register request from JSON");
		assert_eq!(register_request, deserialized_register_request);
	}

	#[test]
	fn get_reference_time_request_should_serialize_and_deserialize() {
		let get_reference_time_request = ClientRequest::GetReferenceTime;
		let json = serde_json::to_string(&get_reference_time_request)
			.expect("Failed to serialize GetReferenceTime request to JSON");
		assert_eq!(r#"{"type":"get_reference_time"}"#, json);

		let deserialized_get_reference_time_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize GetReferenceTime request from JSON");
		assert_eq!(get_reference_time_request, deserialized_get_reference_time_request);
	}

	#[test]
	fn insert_medium_request_should_serialize_and_deserialize() {
		let insert_medium_request = ClientRequest::InsertMedium {
			name: "Blues Brothers".to_string(),
			length_in_milliseconds: 8520000,
		};
		let json =
			serde_json::to_string(&insert_medium_request).expect("Failed to serialize InsertMedium request to JSON");
		assert_eq!(
			r#"{"type":"insert_medium","name":"Blues Brothers","length_in_milliseconds":8520000}"#,
			json
		);

		let deserialized_insert_medium_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize InsertMedium request from JSON");
		assert_eq!(insert_medium_request, deserialized_insert_medium_request);
	}

	#[test]
	fn play_request_should_serialize_and_deserialize() {
		let play_request = ClientRequest::Play {
			skipped: false,
			start_time_in_milliseconds: -1337,
		};
		let json = serde_json::to_string(&play_request).expect("Failed to serialize Play request to JSON");
		assert_eq!(
			r#"{"type":"play","skipped":false,"start_time_in_milliseconds":-1337}"#,
			json
		);

		let deserialized_play_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Play request from JSON");
		assert_eq!(play_request, deserialized_play_request);
	}

	#[test]
	fn pause_request_should_serialize_and_deserialize() {
		let pause_request = ClientRequest::Pause {
			skipped: false,
			position_in_milliseconds: 42,
		};
		let json = serde_json::to_string(&pause_request).expect("Failed to serialize Pause request to JSON");
		assert_eq!(
			r#"{"type":"pause","skipped":false,"position_in_milliseconds":42}"#,
			json
		);

		let deserialized_pause_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Pause request from JSON");
		assert_eq!(pause_request, deserialized_pause_request);
	}

	#[test]
	fn chat_response_should_serialize_and_deserialize() {
		let chat_response = ServerResponse::Chat {
			sender_id: ClientId::from(42),
			sender_name: "Hedwig".to_string(),
			message: "hello".to_string(),
		};
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
		let joined_response = ServerResponse::Joined {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		};
		let json = serde_json::to_string(&joined_response).expect("Failed to serialize Joined response to JSON");
		assert_eq!(r#"{"type":"joined","id":42,"name":"Hedwig"}"#, json);

		let deserialized_joined_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Joined response from JSON");
		assert_eq!(joined_response, deserialized_joined_response);
	}

	#[test]
	fn left_response_should_serialize_and_deserialize() {
		let left_response = ServerResponse::Left {
			id: ClientId::from(42),
			name: "Hedwig".to_string(),
		};
		let json = serde_json::to_string(&left_response).expect("Failed to serialize Left response to JSON");
		assert_eq!(r#"{"type":"left","id":42,"name":"Hedwig"}"#, json);

		let deserialized_left_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Left response from JSON");
		assert_eq!(left_response, deserialized_left_response);
	}

	#[test]
	fn hello_response_without_medium_should_serialize_and_deserialize() {
		let hello_response = ServerResponse::Hello {
			id: 42.into(),
			current_medium: None,
		};
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(r#"{"type":"hello","id":42,"current_medium":null}"#, json);

		let deserialized_hello_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn hello_response_with_medium_should_serialize_and_deserialize() {
		let hello_response = ServerResponse::Hello {
			id: 42.into(),
			current_medium: Some(MediumResponse::FixedLength {
				name: "WarGames".to_string(),
				length_in_milliseconds: Duration::minutes(114).num_milliseconds() as u64,
				playback_state: PlaybackStateResponse::Paused {
					position_in_milliseconds: 0,
				},
			}),
		};
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
		let reference_time_response = ServerResponse::ReferenceTime { milliseconds: 1337 };
		let json = serde_json::to_string(&reference_time_response)
			.expect("Failed to serialize ReferenceTime response to JSON");
		assert_eq!(r#"{"type":"reference_time","milliseconds":1337}"#, json);

		let deserialized_reference_time_response: ServerResponse =
			serde_json::from_str(&json).expect("Failed to deserialize ReferenceTime response from JSON");
		assert_eq!(reference_time_response, deserialized_reference_time_response);
	}

	#[test]
	fn medium_inserted_response_should_serialize_and_deserialize() {
		let medium_inserted_response = ServerResponse::MediumInserted {
			inserted_by_name: "Squirrel".to_string(),
			inserted_by_id: ClientId::from(42),
			name: "The Acorn".to_string(),
			length_in_milliseconds: 20 * 60 * 1000,
		};
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
		let playback_state_changed_response = ServerResponse::PlaybackStateChanged {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Playing {
				start_time_in_milliseconds: -1337,
			},
		};
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
		let playback_state_changed_response = ServerResponse::PlaybackStateChanged {
			changed_by_name: "Alice".to_string(),
			changed_by_id: ClientId::from(0),
			skipped: false,
			playback_state: PlaybackStateResponse::Paused {
				position_in_milliseconds: 42,
			},
		};
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
		let invalid_format_error_response = ServerResponse::Error {
			error: ErrorResponse::InvalidFormat,
			message: "�".to_string(),
		};
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
		let invalid_operation_error_response = ServerResponse::Error {
			error: ErrorResponse::InvalidOperation,
			message: "I'm a teapot.".to_string(),
		};
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
		let internal_server_error_error_response = ServerResponse::Error {
			error: ErrorResponse::InternalServerError,
			message: "I've found a bug crawling around my circuits.".to_string(),
		};
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
