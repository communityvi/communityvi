use anyhow::bail;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub fn axum_websocket_message_to_tungstenite_message(axum_message: axum::extract::ws::Message) -> tungstenite::Message {
	use axum::extract::ws::CloseFrame;
	use axum::extract::ws::Message::*;

	match axum_message {
		Text(text) => tungstenite::Message::Text(text),
		Binary(data) => tungstenite::Message::Binary(data),
		Ping(data) => tungstenite::Message::Ping(data),
		Pong(data) => tungstenite::Message::Pong(data),
		Close(Some(CloseFrame { code, reason })) => {
			tungstenite::Message::Close(Some(tungstenite::protocol::CloseFrame {
				code: code.into(),
				reason,
			}))
		}
		Close(None) => tungstenite::Message::Close(None),
	}
}

pub fn tungstenite_message_to_axum_websocket_message(
	tungstenite_message: tungstenite::Message,
) -> anyhow::Result<axum::extract::ws::Message> {
	use axum::extract::ws;
	use tungstenite::Message::*;

	Ok(match tungstenite_message {
		Text(text) => ws::Message::Text(text),
		Binary(data) => ws::Message::Binary(data),
		Ping(data) => ws::Message::Ping(data),
		Pong(data) => ws::Message::Pong(data),
		Close(Some(CloseFrame { code, reason })) => ws::Message::Close(Some(ws::CloseFrame {
			code: code.into(),
			reason,
		})),
		Close(None) => ws::Message::Close(None),
		Frame(_frame) => bail!("Websocket frames are not supported by axum at the moment"),
	})
}
