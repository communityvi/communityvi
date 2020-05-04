use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub mod client_request;
pub mod server_response;

pub trait Message: Clone + Debug + DeserializeOwned + Serialize + PartialEq {}

pub type WebSocketMessage = tokio_tungstenite::tungstenite::Message;

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

fn deserialize_message_from_str<MessageType: Message>(json: &str) -> Result<MessageType, MessageError> {
	serde_json::from_str(json).map_err(|error| MessageError::DeserializationFailed {
		error: error.to_string(),
		json: json.to_string(),
	})
}
