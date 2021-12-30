use std::fmt::Debug;
use thiserror::Error;

pub mod client_request;
pub mod outgoing;

pub type WebSocketMessage = tokio_tungstenite::tungstenite::Message;

#[derive(Error, Debug)]
pub enum MessageError {
	#[error("Failed to deserialize message with error: '{}'; Message was '{}'", .error, .json)]
	DeserializationFailed { error: String, json: String },
	#[error("Wrong websocket message type. Epected text, got: {0:?}")]
	WrongMessageType(WebSocketMessage),
}
