use chrono::{DateTime, Duration, Utc};
use serde_derive::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OrderedMessage {
	pub number: u64,
	#[serde(flatten)]
	pub message: Message,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Message {
	/// Server time the media playback starts at
	Offset(OffsetMessage),
	/// The server reference time value at the given point in Utc time.
	ServerTime(ServerTimeMessage),
	Ping(TextMessage),
	Pong(TextMessage),
	Chat(TextMessage),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct OffsetMessage {
	/// Server time in milliseconds when the playback of the medium has started
	#[serde(with = "millisecond_duration")]
	pub offset: Duration,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ServerTimeMessage {
	/// Monotonic time in milliseconds the server uses for synchronising playback
	#[serde(with = "millisecond_duration")]
	pub server_time: Duration,
	/// Real time in UTC where the given server time belongs to.
	#[serde(with = "millisecond_timestamp")]
	pub real_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TextMessage {
	pub text: String,
}

pub type WebSocketMessage = warp::filters::ws::Message;

// see https://serde.rs/custom-date-format.html
mod millisecond_timestamp {
	use chrono::{DateTime, LocalResult, TimeZone, Utc};
	use serde::{self, Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(date_time: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let timestamp = date_time.timestamp_millis();
		serializer.serialize_i64(timestamp)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let timestamp = i64::deserialize(deserializer)?;
		let date_time_result = Utc.timestamp_millis_opt(timestamp);
		let date_time = match date_time_result {
			LocalResult::Single(date_time) => date_time,
			_ => {
				return Err(serde::de::Error::custom(format!(
					"Invalid millisecond timestamp: {}",
					timestamp
				)))
			}
		};
		Ok(date_time)
	}
}

// see https://serde.rs/custom-date-format.html
mod millisecond_duration {
	use chrono::Duration;
	use serde::{self, Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let timestamp = duration.num_milliseconds();
		serializer.serialize_i64(timestamp)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
	where
		D: Deserializer<'de>,
	{
		let timestamp = i64::deserialize(deserializer)?;
		Ok(Duration::milliseconds(timestamp))
	}
}

impl From<Message> for WebSocketMessage {
	fn from(message: Message) -> Self {
		let json = serde_json::to_string(&message).expect("Failed to serialize Message to JSON.");
		WebSocketMessage::text(json)
	}
}

impl From<OrderedMessage> for WebSocketMessage {
	fn from(broadcast_message: OrderedMessage) -> Self {
		let json = serde_json::to_string(&broadcast_message).expect("Failed to serialize BroadcastMessage to JSON.");
		WebSocketMessage::text(json)
	}
}

#[derive(Debug)]
pub enum MessageError {
	/// (error_message, message_content)
	DeserializationFailed(String, String),
	WrongMessageType(WebSocketMessage),
}

impl Display for MessageError {
	fn fmt(&self, formatter: &mut Formatter) -> Result<(), std::fmt::Error> {
		match self {
			MessageError::DeserializationFailed(error_message, message) => {
				write!(formatter, "Invalid message: {}; {}", error_message, message)
			}
			MessageError::WrongMessageType(message) => {
				write!(formatter, "Wrong message type. Expected text, got: {:?}", message)
			}
		}
	}
}

impl Error for MessageError {}

impl TryFrom<&str> for Message {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, Self::Error> {
		serde_json::from_str(&json)
			.map_err(|error| MessageError::DeserializationFailed(error.to_string(), json.to_string()))
	}
}

impl TryFrom<&str> for OrderedMessage {
	type Error = MessageError;

	fn try_from(json: &str) -> Result<Self, Self::Error> {
		serde_json::from_str(&json)
			.map_err(|error| MessageError::DeserializationFailed(error.to_string(), json.to_string()))
	}
}

impl TryFrom<WebSocketMessage> for Message {
	type Error = MessageError;

	fn try_from(websocket_message: WebSocketMessage) -> Result<Self, Self::Error> {
		let json = websocket_message
			.to_str()
			.map_err(|()| MessageError::WrongMessageType(websocket_message.clone()))?;
		json.try_into()
	}
}

impl TryFrom<WebSocketMessage> for OrderedMessage {
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
	use chrono::TimeZone;

	#[test]
	fn offset_message_should_serialize_and_deserialize() {
		let offset_message = Message::Offset(OffsetMessage {
			offset: Duration::milliseconds(42),
		});
		let json = serde_json::to_string(&offset_message).expect("Failed to serialize OffsetMessage to JSON.");
		assert_eq!(json, r#"{"type":"offset","offset":42}"#);

		let deserialized_offset_message: Message =
			serde_json::from_str(&json).expect("Failed to deserialize OffsetMessage from JSON.");
		assert_eq!(deserialized_offset_message, offset_message);
	}

	#[test]
	fn server_time_message_should_serialize_and_deserialize() {
		let server_time_message = Message::ServerTime(ServerTimeMessage {
			server_time: Duration::milliseconds(1337),
			real_time: Utc.ymd(2019, 7, 21).and_hms_milli(13, 37, 42, 666),
		});
		let json = serde_json::to_string(&server_time_message).expect("Failed to serialize ServerTimeMessage to JSON.");
		assert_eq!(
			json,
			r#"{"type":"server_time","server_time":1337,"real_time":1563716262666}"#
		);

		let deserialized_server_time_message: Message =
			serde_json::from_str(&json).expect("Failed to deserialize ServerTimeMessage from JSON");
		assert_eq!(deserialized_server_time_message, server_time_message);
	}

	#[test]
	fn ping_message_should_serialize_and_deserialize() {
		let ping_message = Message::Ping(TextMessage { text: "hello".into() });
		let json = serde_json::to_string(&ping_message).expect("Failed to serialize PingMessage to JSON");
		assert_eq!(json, r#"{"type":"ping","text":"hello"}"#);

		let deserialized_ping_message: Message =
			serde_json::from_str(&json).expect("Failed to deserialize PingMessage from JSON");
		assert_eq!(deserialized_ping_message, ping_message);
	}

	#[test]
	fn pong_message_should_serialize_and_deserialize() {
		let pong_message = Message::Pong(TextMessage { text: "hello".into() });
		let json = serde_json::to_string(&pong_message).expect("Failed to serialize PongMessage to JSON");
		assert_eq!(json, r#"{"type":"pong","text":"hello"}"#);

		let deserialized_pong_message: Message =
			serde_json::from_str(&json).expect("Failed to deserialize PongMessage from JSON");
		assert_eq!(deserialized_pong_message, pong_message);
	}

	#[test]
	fn chat_message_should_serialize_and_deserialize() {
		let chat_message = Message::Chat(TextMessage { text: "hello".into() });
		let json = serde_json::to_string(&chat_message).expect("Failed to serialize ChatMessage to JSON");
		assert_eq!(json, r#"{"type":"chat","text":"hello"}"#);

		let deserialized_chat_message: Message =
			serde_json::from_str(&json).expect("Failed to deserialize ChatMessage from JSON");
		assert_eq!(deserialized_chat_message, chat_message);
	}

	#[test]
	fn ordered_message_should_serialize_and_deserialize() {
		let ordered_message = OrderedMessage {
			number: 12,
			message: Message::Chat(TextMessage {
				text: "announcement".to_string(),
			}),
		};
		let json = serde_json::to_string(&ordered_message).expect("Failed to serialize OrderedMessage to JSON");
		assert_eq!(json, r#"{"number":12,"type":"chat","text":"announcement"}"#);

		let deserialized_ordered_message: OrderedMessage =
			serde_json::from_str(&json).expect("Failed to deserialize BroadcastMessage from JSON");
		assert_eq!(deserialized_ordered_message, ordered_message);
	}
}
