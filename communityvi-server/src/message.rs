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
	Register { name: String },
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
	fn ping_request_should_serialize_and_deserialize() {
		let ping_request = first_message(ClientRequest::Ping);
		let json = serde_json::to_string(&ping_request).expect("Failed to serialize Ping request to JSON");
		assert_eq!(json, r#"{"number":0,"type":"ping"}"#);

		let deserialized_ping_request: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize Ping request from JSON");
		assert_eq!(ping_request, deserialized_ping_request);
	}

	#[test]
	fn pong_request_should_serialize_and_deserialize() {
		let pong_request = first_message(ClientRequest::Pong);
		let json = serde_json::to_string(&pong_request).expect("Failed to serialize Pong request to JSON");
		assert_eq!(json, r#"{"number":0,"type":"pong"}"#);

		let deserialized_pong_request: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize Pong request from JSON");
		assert_eq!(pong_request, deserialized_pong_request);
	}

	#[test]
	fn chat_request_should_serialize_and_deserialize() {
		let chat_request = first_message(ClientRequest::Chat {
			message: "hello".into(),
		});
		let json = serde_json::to_string(&chat_request).expect("Failed to serialize Chat request to JSON");
		assert_eq!(json, r#"{"number":0,"type":"chat","request":"hello"}"#);

		let deserialized_chat_request: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize Chat request from JSON");
		assert_eq!(chat_request, deserialized_chat_request);
	}

	#[test]
	fn register_request_should_serialize_and_deserialize() {
		let register_request = first_message(ClientRequest::Register {
			name: "Ferris".to_string(),
		});
		let json = serde_json::to_string(&register_request).expect("Failed to serialize Register request to JSON");
		assert_eq!(json, r#"{"number":0,"type":"register","name":"Ferris"}"#);

		let deserialized_register_request: OrderedMessage<ClientRequest> =
			serde_json::from_str(&json).expect("Failed to deserialize Register request from JSON");
		assert_eq!(register_request, deserialized_register_request);
	}
}
