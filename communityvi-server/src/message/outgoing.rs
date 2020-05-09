use crate::message::outgoing::broadcast_message::BroadcastMessage;
use crate::message::outgoing::error_message::ErrorMessage;
use crate::message::outgoing::success_message::SuccessMessage;
use crate::message::{MessageError, WebSocketMessage};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub mod broadcast_message;
pub mod error_message;
pub mod success_message;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum OutgoingMessage {
	Success {
		request_id: u64,
		message: SuccessMessage,
	},
	Error {
		request_id: Option<u64>,
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
