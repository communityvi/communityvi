use serde::{Deserialize, Serialize};

use crate::message::{
	deserialize_message_from_str, serialize_message_to_websocket_message, Message, MessageError, WebSocketMessage,
};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ClientRequest {
	Register(RegisterRequest),
	Chat(ChatRequest),
	GetReferenceTime,
	InsertMedium(InsertMediumRequest),
	Play(PlayRequest),
	Pause(PauseRequest),
}

macro_rules! client_request_from_struct {
	($enum_case: ident, $struct_type: ty) => {
		impl From<$struct_type> for ClientRequest {
			fn from(request: $struct_type) -> ClientRequest {
				ClientRequest::$enum_case(request)
			}
		}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RegisterRequest {
	pub name: String,
}

client_request_from_struct!(Register, RegisterRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ChatRequest {
	pub message: String,
}

client_request_from_struct!(Chat, ChatRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct InsertMediumRequest {
	pub name: String,
	pub length_in_milliseconds: u64,
}

client_request_from_struct!(InsertMedium, InsertMediumRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayRequest {
	pub skipped: bool,
	pub start_time_in_milliseconds: i64,
}

client_request_from_struct!(Play, PlayRequest);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PauseRequest {
	pub skipped: bool,
	pub position_in_milliseconds: u64,
}

client_request_from_struct!(Pause, PauseRequest);

impl Message for ClientRequest {}

impl From<&ClientRequest> for WebSocketMessage {
	fn from(request: &ClientRequest) -> Self {
		serialize_message_to_websocket_message(request)
	}
}

impl TryFrom<&str> for ClientRequest {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, Self::Error> {
		deserialize_message_from_str(json)
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

	#[test]
	fn chat_request_should_serialize_and_deserialize() {
		let chat_request = ClientRequest::Chat(ChatRequest {
			message: "hello".into(),
		});
		let json = serde_json::to_string(&chat_request).expect("Failed to serialize Chat request to JSON");
		assert_eq!(r#"{"type":"chat","message":"hello"}"#, json);

		let deserialized_chat_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Chat request from JSON");
		assert_eq!(chat_request, deserialized_chat_request);
	}

	#[test]
	fn register_request_should_serialize_and_deserialize() {
		let register_request = ClientRequest::Register(RegisterRequest {
			name: "Ferris".to_string(),
		});
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
		let insert_medium_request = ClientRequest::InsertMedium(InsertMediumRequest {
			name: "Blues Brothers".to_string(),
			length_in_milliseconds: 8520000,
		});
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
		let play_request = ClientRequest::Play(PlayRequest {
			skipped: false,
			start_time_in_milliseconds: -1337,
		});
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
		let pause_request = ClientRequest::Pause(PauseRequest {
			skipped: false,
			position_in_milliseconds: 42,
		});
		let json = serde_json::to_string(&pause_request).expect("Failed to serialize Pause request to JSON");
		assert_eq!(
			r#"{"type":"pause","skipped":false,"position_in_milliseconds":42}"#,
			json
		);

		let deserialized_pause_request: ClientRequest =
			serde_json::from_str(&json).expect("Failed to deserialize Pause request from JSON");
		assert_eq!(pause_request, deserialized_pause_request);
	}
}
