use serde::{Deserialize, Serialize};

use crate::message::outgoing::error_message::{ErrorMessage, ErrorMessageType};
use crate::message::{MessageError, WebSocketMessage};
use crate::room::medium::fixed_length::FixedLengthMedium;
use crate::room::medium::Medium;
use chrono::Duration;
use js_int::{Int, UInt};
use log::error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClientRequestWithId {
	pub request_id: UInt,
	#[serde(flatten)]
	pub request: ClientRequest,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ClientRequest {
	Register(RegisterRequest),
	Chat(ChatRequest),
	InsertMedium(InsertMediumRequest),
	Play(PlayRequest),
	Pause(PauseRequest),
}

impl ClientRequest {
	pub fn kind(&self) -> &'static str {
		use ClientRequest::*;
		match self {
			Register(_) => "Register",
			Chat(_) => "Chat",
			InsertMedium(_) => "InsertMedium",
			Play(_) => "Play",
			Pause(_) => "Pause",
		}
	}
}

#[allow(dead_code)]
trait RequestConvertible: Into<ClientRequest> {
	fn with_id(self, request_id: UInt) -> ClientRequestWithId {
		ClientRequestWithId {
			request_id,
			request: self.into(),
		}
	}
}

impl RequestConvertible for ClientRequest {}

impl From<ClientRequestWithId> for ClientRequest {
	fn from(client_request: ClientRequestWithId) -> Self {
		client_request.request
	}
}

macro_rules! client_request_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for ClientRequest {
			fn from(request: $struct_type) -> ClientRequest {
				ClientRequest::$enum_case(request)
			}
		}

		impl RequestConvertible for $struct_type {}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegisterRequest {
	pub name: String,
}

client_request_from_struct!(Register, RegisterRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ChatRequest {
	pub message: String,
}

client_request_from_struct!(Chat, ChatRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InsertMediumRequest {
	pub previous_version: UInt,
	pub medium: MediumRequest,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MediumRequest {
	FixedLength { name: String, length_in_milliseconds: UInt },
	Empty,
}

client_request_from_struct!(InsertMedium, InsertMediumRequest);

impl TryFrom<MediumRequest> for Medium {
	type Error = ErrorMessage;

	fn try_from(request: MediumRequest) -> Result<Self, Self::Error> {
		match request {
			MediumRequest::FixedLength {
				name,
				length_in_milliseconds,
			} => {
				if length_in_milliseconds > (UInt::try_from(Duration::days(365).num_milliseconds()).unwrap()) {
					Err(ErrorMessage::builder()
						.error(ErrorMessageType::InvalidFormat)
						.message("Length of a medium must not be larger than one year.".to_string())
						.build())
				} else {
					Ok(FixedLengthMedium::new(name, Duration::milliseconds(i64::from(length_in_milliseconds))).into())
				}
			}
			MediumRequest::Empty => Ok(Medium::Empty),
		}
	}
}

impl From<Medium> for MediumRequest {
	fn from(medium: Medium) -> Self {
		match medium {
			Medium::Empty => MediumRequest::Empty,
			Medium::FixedLength(fixed_length) => MediumRequest::FixedLength {
				name: fixed_length.name,
				length_in_milliseconds: UInt::try_from(fixed_length.length.num_milliseconds()).unwrap(),
			},
		}
	}
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlayRequest {
	pub previous_version: UInt,
	pub skipped: bool,
	pub start_time_in_milliseconds: Int,
}

client_request_from_struct!(Play, PlayRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PauseRequest {
	pub previous_version: UInt,
	pub skipped: bool,
	pub position_in_milliseconds: UInt,
}

client_request_from_struct!(Pause, PauseRequest);

impl From<&ClientRequestWithId> for WebSocketMessage {
	fn from(request: &ClientRequestWithId) -> Self {
		let json = serde_json::to_string(request).expect("Failed to serialize request to JSON.");
		WebSocketMessage::text(json)
	}
}

impl TryFrom<&WebSocketMessage> for ClientRequestWithId {
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RequestIdOnly {
	pub request_id: UInt,
}

impl TryFrom<&WebSocketMessage> for RequestIdOnly {
	type Error = ();

	fn try_from(websocket_message: &WebSocketMessage) -> Result<Self, Self::Error> {
		match websocket_message {
			WebSocketMessage::Text(json) => serde_json::from_str(json)
				.map_err(|error| error!("Error while deserializing websocket message from JSON: {:?}", error)),
			_ => Err(()),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use js_int::{int, uint};

	#[test]
	fn chat_request_should_serialize_and_deserialize() {
		let chat_request = ClientRequest::Chat(ChatRequest {
			message: "hello".into(),
		})
		.with_id(uint!(42));
		let json = serde_json::to_string(&chat_request).expect("Failed to serialize Chat request to JSON");
		assert_eq!(r#"{"request_id":42,"type":"chat","message":"hello"}"#, json);

		let deserialized_chat_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize Chat request from JSON");
		assert_eq!(chat_request, deserialized_chat_request);
	}

	#[test]
	fn register_request_should_serialize_and_deserialize() {
		let register_request = ClientRequest::Register(RegisterRequest {
			name: "Ferris".to_string(),
		})
		.with_id(uint!(42));
		let json = serde_json::to_string(&register_request).expect("Failed to serialize Register request to JSON");
		assert_eq!(r#"{"request_id":42,"type":"register","name":"Ferris"}"#, json);

		let deserialized_register_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize Register request from JSON");
		assert_eq!(register_request, deserialized_register_request);
	}

	#[test]
	fn insert_medium_request_with_fixed_length_medium_should_serialize_and_deserialize() {
		let insert_medium_request = ClientRequest::InsertMedium(InsertMediumRequest {
			medium: MediumRequest::FixedLength {
				name: "Blues Brothers".to_string(),
				length_in_milliseconds: uint!(8_520_000),
			},
			previous_version: uint!(0),
		})
		.with_id(uint!(42));
		let json =
			serde_json::to_string(&insert_medium_request).expect("Failed to serialize InsertMedium request to JSON");
		assert_eq!(
			r#"{"request_id":42,"type":"insert_medium","previous_version":0,"medium":{"type":"fixed_length","name":"Blues Brothers","length_in_milliseconds":8520000}}"#,
			json
		);

		let deserialized_insert_medium_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize InsertMedium request from JSON");
		assert_eq!(insert_medium_request, deserialized_insert_medium_request);
	}

	#[test]
	fn insert_medium_request_with_empty_medium_should_serialize_and_deserialize() {
		let eject_medium_request = ClientRequest::InsertMedium(InsertMediumRequest {
			previous_version: uint!(0),
			medium: MediumRequest::Empty,
		})
		.with_id(uint!(42));
		let json =
			serde_json::to_string(&eject_medium_request).expect("Failed to serialize InsertMedium request to JSON");
		assert_eq!(
			r#"{"request_id":42,"type":"insert_medium","previous_version":0,"medium":{"type":"empty"}}"#,
			json
		);

		let deserialized_eject_medium_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize InsertMedium request from JSON");
		assert_eq!(eject_medium_request, deserialized_eject_medium_request);
	}

	#[test]
	fn play_request_should_serialize_and_deserialize() {
		let play_request = ClientRequest::Play(PlayRequest {
			previous_version: uint!(0),
			skipped: false,
			start_time_in_milliseconds: int!(-1337),
		})
		.with_id(uint!(42));
		let json = serde_json::to_string(&play_request).expect("Failed to serialize Play request to JSON");
		assert_eq!(
			r#"{"request_id":42,"type":"play","previous_version":0,"skipped":false,"start_time_in_milliseconds":-1337}"#,
			json
		);

		let deserialized_play_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize Play request from JSON");
		assert_eq!(play_request, deserialized_play_request);
	}

	#[test]
	fn pause_request_should_serialize_and_deserialize() {
		let pause_request = ClientRequest::Pause(PauseRequest {
			previous_version: uint!(0),
			skipped: false,
			position_in_milliseconds: uint!(42),
		})
		.with_id(uint!(42));
		let json = serde_json::to_string(&pause_request).expect("Failed to serialize Pause request to JSON");
		assert_eq!(
			r#"{"request_id":42,"type":"pause","previous_version":0,"skipped":false,"position_in_milliseconds":42}"#,
			json
		);

		let deserialized_pause_request: ClientRequestWithId =
			serde_json::from_str(&json).expect("Failed to deserialize Pause request from JSON");
		assert_eq!(pause_request, deserialized_pause_request);
	}

	#[test]
	fn request_id_only_should_serialize_and_deserialize() {
		let request_id_only = RequestIdOnly { request_id: uint!(42) };
		let json = serde_json::to_string(&request_id_only).expect("Failed to serialize RequestIdOnly to JSON");
		assert_eq!(r#"{"request_id":42}"#, json);

		let deserialized_request_id_only: RequestIdOnly =
			serde_json::from_str(&json).expect("Failed to deserialize RequestIdOnly from JSON");
		assert_eq!(request_id_only, deserialized_request_id_only);
	}

	#[test]
	fn request_id_only_should_deserialize_even_with_additional_fields() {
		let json = r#"{"request_id":42,"garbage":"smelly"}"#;
		let request_id_only: RequestIdOnly =
			serde_json::from_str(json).expect("Failed to deserialize RequestIdOnly from JSON");

		assert_eq!(request_id_only.request_id, uint!(42));
	}
}
