use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OrderedMessage<MessageType: Message> {
	pub number: u64,
	#[serde(bound(deserialize = "MessageType: Message"))]
	#[serde(flatten)]
	pub message: MessageType,
}

pub trait Message: Clone + Debug + DeserializeOwned + Serialize + PartialEq {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ClientRequest {
	Ping,
	Pong,
	Chat { message: String },
}

impl Message for ClientRequest {}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ServerResponse {
	Ping,
	Pong,
	Chat { message: String },
}

impl Message for ServerResponse {}

pub type WebSocketMessage = warp::filters::ws::Message;

impl<MessageType: Message> From<&OrderedMessage<MessageType>> for WebSocketMessage {
	fn from(message: &OrderedMessage<MessageType>) -> Self {
		let json = serde_json::to_string(message).expect("Failed to serialize message to JSON.");
		WebSocketMessage::text(json)
	}
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

impl<MessageType: Message> TryFrom<&str> for OrderedMessage<MessageType> {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, Self::Error> {
		serde_json::from_str(json).map_err(|error| MessageError::DeserializationFailed {
			error: error.to_string(),
			json: json.to_string(),
		})
	}
}

impl<MessageType: Message> TryFrom<WebSocketMessage> for OrderedMessage<MessageType> {
	type Error = MessageError;

	fn try_from(websocket_message: WebSocketMessage) -> Result<Self, Self::Error> {
		let json = websocket_message
			.to_str()
			.map_err(|()| MessageError::WrongMessageType(websocket_message.clone()))?;
		json.try_into()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn first_message<MessageType: Message>(message: MessageType) -> OrderedMessage<MessageType> {
		OrderedMessage { number: 0, message }
	}

	#[test]
	fn ping_message_should_serialize_and_deserialize() {
		let ping_message = first_message(ClientRequest::Ping);
		let json = serde_json::to_string(&ping_message).expect("Failed to serialize PingMessage to JSON");
		assert_eq!(json, r#"{"number":0,"type":"ping"}"#);

		let deserialized_ping_message: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize PingMessage from JSON");
		assert_eq!(deserialized_ping_message, ping_message);
	}

	#[test]
	fn pong_message_should_serialize_and_deserialize() {
		let pong_message = first_message(ClientRequest::Pong);
		let json = serde_json::to_string(&pong_message).expect("Failed to serialize PongMessage to JSON");
		assert_eq!(json, r#"{"number":0,"type":"pong"}"#);

		let deserialized_pong_message: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize PongMessage from JSON");
		assert_eq!(deserialized_pong_message, pong_message);
	}

	#[test]
	fn chat_message_should_serialize_and_deserialize() {
		let chat_message = first_message(ClientRequest::Chat {
			message: "hello".into(),
		});
		let json = serde_json::to_string(&chat_message).expect("Failed to serialize ChatMessage to JSON");
		assert_eq!(json, r#"{"number":0,"type":"chat","message":"hello"}"#);

		let deserialized_chat_message: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize ChatMessage from JSON");
		assert_eq!(deserialized_chat_message, chat_message);
	}
}
