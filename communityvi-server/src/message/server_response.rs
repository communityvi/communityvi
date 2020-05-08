use serde::{Deserialize, Serialize};

use crate::message::{MessageError, WebSocketMessage};
use crate::room::client_id::ClientId;
use crate::room::state::medium::playback_state::PlaybackState;
use crate::room::state::medium::{Medium, SomeMedium};
use std::convert::TryFrom;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerResponseWithId {
	pub request_id: Option<u64>,
	#[serde(flatten)]
	pub response: ServerResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ServerResponse {
	Hello(HelloResponse),
	ReferenceTime(ReferenceTimeResponse),
	Error(ErrorResponse),
}

pub trait ResponseConvertible: Into<ServerResponse> {
	fn with_id(self, request_id: u64) -> ServerResponseWithId {
		ServerResponseWithId {
			request_id: Some(request_id),
			response: self.into(),
		}
	}
}

impl ResponseConvertible for ServerResponse {}

macro_rules! server_response_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for ServerResponse {
			fn from(response: $struct_type) -> ServerResponse {
				ServerResponse::$enum_case(response)
			}
		}

		impl ResponseConvertible for $struct_type {}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HelloResponse {
	pub id: ClientId,
	pub current_medium: Option<MediumResponse>,
}

server_response_from_struct!(Hello, HelloResponse);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ReferenceTimeResponse {
	pub milliseconds: u64,
}

server_response_from_struct!(ReferenceTime, ReferenceTimeResponse);

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

impl From<&ServerResponseWithId> for WebSocketMessage {
	fn from(response: &ServerResponseWithId) -> Self {
		let json = serde_json::to_string(response).expect("Failed to serialize response to JSON.");
		WebSocketMessage::text(json)
	}
}

impl TryFrom<&WebSocketMessage> for ServerResponseWithId {
	type Error = MessageError;

	fn try_from(websocket_message: &WebSocketMessage) -> Result<Self, MessageError> {
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

#[cfg(test)]
mod test {
	use super::*;
	use chrono::Duration;

	#[test]
	fn hello_response_without_medium_should_serialize_and_deserialize() {
		let hello_response = HelloResponse {
			id: 42.into(),
			current_medium: None,
		}
		.with_id(1337);
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"hello","id":42,"current_medium":null}"#,
			json
		);

		let deserialized_hello_response: ServerResponseWithId =
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
		})
		.with_id(1337);
		let json = serde_json::to_string(&hello_response).expect("Failed to serialize Hello response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"hello","id":42,"current_medium":{"type":"fixed_length","name":"WarGames","length_in_milliseconds":6840000,"playback_state":{"type":"paused","position_in_milliseconds":0}}}"#,
			json
		);

		let deserialized_hello_response: ServerResponseWithId =
			serde_json::from_str(&json).expect("Failed to deserialize Hello response from JSON");
		assert_eq!(hello_response, deserialized_hello_response);
	}

	#[test]
	fn reference_time_response_should_serialize_and_deserialize() {
		let reference_time_response =
			ServerResponse::ReferenceTime(ReferenceTimeResponse { milliseconds: 1337 }).with_id(1337);
		let json = serde_json::to_string(&reference_time_response)
			.expect("Failed to serialize ReferenceTime response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"reference_time","milliseconds":1337}"#,
			json
		);

		let deserialized_reference_time_response: ServerResponseWithId =
			serde_json::from_str(&json).expect("Failed to deserialize ReferenceTime response from JSON");
		assert_eq!(reference_time_response, deserialized_reference_time_response);
	}

	#[test]
	fn invalid_format_error_response_should_serialize_and_deserialize() {
		let invalid_format_error_response = ServerResponse::Error(ErrorResponse {
			error: ErrorResponseType::InvalidFormat,
			message: "�".to_string(),
		})
		.with_id(1337);
		let json = serde_json::to_string(&invalid_format_error_response)
			.expect("Failed to serialize InvalidFormat error response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"error","error":"invalid_format","message":"�"}"#,
			json
		);

		let deserialized_invalid_format_error_response: ServerResponseWithId =
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
		})
		.with_id(1337);
		let json = serde_json::to_string(&invalid_operation_error_response)
			.expect("Failed to serialize InvalidOperation error response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"error","error":"invalid_operation","message":"I'm a teapot."}"#,
			json
		);

		let deserialized_invalid_operation_error_response: ServerResponseWithId =
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
		})
		.with_id(1337);
		let json = serde_json::to_string(&internal_server_error_error_response)
			.expect("Failed to serialize InternalServerError error response to JSON");
		assert_eq!(
			r#"{"request_id":1337,"type":"error","error":"internal_server_error","message":"I've found a bug crawling around my circuits."}"#,
			json
		);

		let deserialized_internal_server_error_error_response: ServerResponseWithId =
			serde_json::from_str(&json).expect("Failed to deserialize InternalServerError error response from JSON");
		assert_eq!(
			internal_server_error_error_response,
			deserialized_internal_server_error_error_response
		);
	}

	#[test]
	fn server_response_without_request_id_should_serialize_as_null() {
		let response_without_id = ServerResponseWithId {
			request_id: None,
			response: ErrorResponse {
				error: ErrorResponseType::InvalidFormat,
				message: "No request id".to_string(),
			}
			.into(),
		};
		let json =
			serde_json::to_string(&response_without_id).expect("Failed to serialize response without request id");
		let expected_json = r#"{"request_id":null,"type":"error","error":"invalid_format","message":"No request id"}"#;

		assert_eq!(json, expected_json);
	}
}
