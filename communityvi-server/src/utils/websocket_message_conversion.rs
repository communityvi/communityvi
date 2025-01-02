use anyhow::bail;
use std::borrow::Cow;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Utf8Bytes;

pub fn axum_websocket_message_to_tungstenite_message(axum_message: axum::extract::ws::Message) -> tungstenite::Message {
	use axum::extract::ws::CloseFrame;
	use axum::extract::ws::Message::*;

	match axum_message {
		Text(text) => tungstenite::Message::Text(text.into()),
		Binary(data) => tungstenite::Message::Binary(data.into()),
		Ping(data) => tungstenite::Message::Ping(data.into()),
		Pong(data) => tungstenite::Message::Pong(data.into()),
		Close(Some(CloseFrame { code, reason })) => {
			tungstenite::Message::Close(Some(tungstenite::protocol::CloseFrame {
				code: code.into(),
				reason: match reason {
					Cow::Borrowed(reason) => Utf8Bytes::from_static(reason),
					Cow::Owned(reason) => reason.into(),
				},
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
		Text(text) => ws::Message::Text(text.as_str().to_owned()),
		Binary(data) => ws::Message::Binary(data.into()),
		Ping(data) => ws::Message::Ping(data.into()),
		Pong(data) => ws::Message::Pong(data.into()),
		Close(Some(CloseFrame { code, reason })) => ws::Message::Close(Some(ws::CloseFrame {
			code: code.into(),
			reason: reason.as_str().to_owned().into(),
		})),
		Close(None) => ws::Message::Close(None),
		Frame(_frame) => bail!("Websocket frames are not supported by axum at the moment"),
	})
}
