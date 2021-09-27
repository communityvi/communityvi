use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub fn rweb_websocket_message_to_tungstenite_message(rweb_message: rweb::ws::Message) -> tungstenite::Message {
	if let Ok(text) = rweb_message.to_str() {
		assert!(rweb_message.is_text());
		return tungstenite::Message::Text(text.into());
	}

	if rweb_message.is_binary() {
		return tungstenite::Message::Binary(rweb_message.into_bytes());
	}

	if rweb_message.is_ping() {
		return tungstenite::Message::Ping(rweb_message.into_bytes());
	}

	if rweb_message.is_pong() {
		return tungstenite::Message::Pong(rweb_message.into_bytes());
	}

	if rweb_message.is_close() {
		return match rweb_message.close_frame() {
			Some((code, reason)) => tungstenite::Message::Close(Some(CloseFrame {
				code: code.into(),
				reason: reason.to_string().into(),
			})),
			None => tungstenite::Message::Close(None),
		};
	}

	unreachable!("Unknown type of rweb message: {:?}", rweb_message);
}

pub fn tungstenite_message_to_rweb_websocket_message(tungstenite_message: tungstenite::Message) -> rweb::ws::Message {
	use tungstenite::Message::*;
	match tungstenite_message {
		Text(text) => rweb::ws::Message::text(text),
		Binary(data) => rweb::ws::Message::binary(data),
		Ping(data) => rweb::ws::Message::ping(data),
		Pong(data) => rweb::ws::Message::pong(data),
		Close(Some(frame)) => rweb::ws::Message::close_with(frame.code, frame.reason),
		Close(None) => rweb::ws::Message::close(),
	}
}
