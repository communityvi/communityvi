use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, TypedBuilder)]
pub struct ErrorMessage {
	pub error: ErrorMessageType,
	pub message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorMessageType {
	InvalidFormat,
	InvalidOperation,
	NoMedium,
	InternalServerError,
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn invalid_format_error_message_should_serialize_and_deserialize() {
		let invalid_format_error_message = ErrorMessage::builder()
			.error(ErrorMessageType::InvalidFormat)
			.message("�".to_string())
			.build();
		let json = serde_json::to_string(&invalid_format_error_message)
			.expect("Failed to serialize InvalidFormat error message to JSON");
		assert_eq!(r#"{"error":"invalid_format","message":"�"}"#, json);

		let deserialized_invalid_format_error_message: ErrorMessage =
			serde_json::from_str(&json).expect("Failed to deserialize InvalidFormat error message from JSON");
		assert_eq!(invalid_format_error_message, deserialized_invalid_format_error_message);
	}

	#[test]
	fn invalid_operation_error_message_should_serialize_and_deserialize() {
		let invalid_operation_error_message = ErrorMessage::builder()
			.error(ErrorMessageType::InvalidOperation)
			.message("I'm a teapot.".to_string())
			.build();
		let json = serde_json::to_string(&invalid_operation_error_message)
			.expect("Failed to serialize InvalidOperation error message to JSON");
		assert_eq!(r#"{"error":"invalid_operation","message":"I'm a teapot."}"#, json);

		let deserialized_invalid_operation_error_message: ErrorMessage =
			serde_json::from_str(&json).expect("Failed to deserialize InvalidOperation error message from JSON");
		assert_eq!(
			invalid_operation_error_message,
			deserialized_invalid_operation_error_message
		);
	}

	#[test]
	fn internal_server_error_message_should_serialize_and_deserialize() {
		let internal_server_error_error_message = ErrorMessage::builder()
			.error(ErrorMessageType::InternalServerError)
			.message("I've found a bug crawling around my circuits.".to_string())
			.build();
		let json = serde_json::to_string(&internal_server_error_error_message)
			.expect("Failed to serialize InternalServerError error message to JSON");
		assert_eq!(
			r#"{"error":"internal_server_error","message":"I've found a bug crawling around my circuits."}"#,
			json
		);

		let deserialized_internal_server_error_error_message: ErrorMessage =
			serde_json::from_str(&json).expect("Failed to deserialize InternalServerError error message from JSON");
		assert_eq!(
			internal_server_error_error_message,
			deserialized_internal_server_error_error_message
		);
	}
}
