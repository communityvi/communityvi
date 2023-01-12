use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::{MessageError, WebSocketMessage};
use js_int::UInt;
use serde::{Deserialize, Serialize};

pub mod broadcast_message;
pub mod error_message;
pub mod success_message;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum OutgoingMessage {
	Success {
		request_id: UInt,
	},
	Error {
		request_id: Option<UInt>,
		message: ErrorMessage,
	},
	Broadcast {
		message: BroadcastMessage,
	},
}

impl From<&OutgoingMessage> for WebSocketMessage {
	fn from(response: &OutgoingMessage) -> Self {
		let json = serde_json::to_string(response).expect("Failed to serialize response to JSON.");
		WebSocketMessage::text(json)
	}
}

impl TryFrom<&WebSocketMessage> for OutgoingMessage {
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
	use crate::message::outgoing::broadcast_message::{ClientJoinedBroadcast, Participant};
	use crate::message::outgoing::error_message::ErrorMessageType;
	use crate::room::session_id::SessionId;
	use js_int::uint;
	use std::collections::BTreeSet;

	#[test]
	fn success_message_should_serialize_and_deserialize() {
		let success_message = OutgoingMessage::Success { request_id: uint!(42) };
		let json = serde_json::to_string(&success_message).expect("Failed to serialize Success message to JSON");
		assert_eq!(r#"{"type":"success","request_id":42}"#, json);

		let deserialized_success_message: OutgoingMessage =
			serde_json::from_str(&json).expect("Failed to deserialize Success message from JSON");
		assert_eq!(success_message, deserialized_success_message);
	}

	#[test]
	fn error_message_with_request_id_should_serialize_and_deserialize() {
		let error_message = OutgoingMessage::Error {
			request_id: Some(uint!(42)),
			message: ErrorMessage::builder()
				.error(ErrorMessageType::InternalServerError)
				.message("No medium".to_string())
				.build(),
		};
		let json = serde_json::to_string(&error_message).expect("Failed to serialize error message to JSON");
		assert_eq!(
			r#"{"type":"error","request_id":42,"message":{"error":"internal_server_error","message":"No medium"}}"#,
			json
		);

		let deserialized_error_message: OutgoingMessage =
			serde_json::from_str(&json).expect("Failed to deserialize error message from JSON");
		assert_eq!(error_message, deserialized_error_message);
	}

	#[test]
	fn error_message_without_request_id_should_serialize_and_deserialize() {
		let error_message = OutgoingMessage::Error {
			request_id: None,
			message: ErrorMessage::builder()
				.error(ErrorMessageType::InvalidFormat)
				.message("Missing request_id".to_string())
				.build(),
		};
		let json = serde_json::to_string(&error_message).expect("Failed to serialize error message to JSON");
		assert_eq!(
			r#"{"type":"error","request_id":null,"message":{"error":"invalid_format","message":"Missing request_id"}}"#,
			json
		);

		let deserialized_error_message: OutgoingMessage =
			serde_json::from_str(&json).expect("Failed to deserialize error message from JSON");
		assert_eq!(error_message, deserialized_error_message);
	}

	#[test]
	fn broadcast_message_should_serialize_and_deserialize() {
		let nena = Participant::new(SessionId::from(98), "Nena".to_string());
		let luftballons = Participant::new(SessionId::from(99), "Luftballons".to_string());
		let broadcast_message = OutgoingMessage::Broadcast {
			message: BroadcastMessage::ClientJoined(ClientJoinedBroadcast {
				id: SessionId::from(99),
				name: "Luftballons".to_string(),
				participants: BTreeSet::from_iter([nena, luftballons]),
			}),
		};
		let json = serde_json::to_string(&broadcast_message).expect("Failed to serialize broadcast message to JSON");
		assert_eq!(
			r#"{"type":"broadcast","message":{"type":"client_joined","id":99,"name":"Luftballons","participants":[{"id":99,"name":"Luftballons"},{"id":98,"name":"Nena"}]}}"#,
			json
		);

		let deserialized_broadcast_message: OutgoingMessage =
			serde_json::from_str(&json).expect("Failed to deserialize broadcast message from JSON");
		assert_eq!(broadcast_message, deserialized_broadcast_message);
	}
}
