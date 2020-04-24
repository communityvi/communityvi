/*
 * This code has been derived from the Gotham project.
 * https://github.com/gotham-rs/gotham/blob/4c99941b3f986c791b5fbfce13dac244756abf66/examples/websocket/src/ws.rs
 * License as follows:
 *
 * > Copyright 2017 The Gotham Project Developers.
 * >
 * > Permission is hereby granted, free of charge, to any person obtaining a copy of
 * > this software and associated documentation files (the "Software"), to deal in
 * > the Software without restriction, including without limitation the rights to
 * > use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
 * > of the Software, and to permit persons to whom the Software is furnished to do
 * > so, subject to the following conditions:
 * >
 * > The above copyright notice and this permission notice shall be included in all
 * > copies or substantial portions of the Software.
 * >
 * > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * > IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * > FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * > LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * > SOFTWARE.
 */
use base64;
use gotham::hyper::header::{HeaderValue, CONNECTION, UPGRADE};
use gotham::hyper::{upgrade::Upgraded, Body, HeaderMap, Response, StatusCode};
use log::error;
use sha1::Sha1;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{tungstenite, WebSocketStream};

use std::future::Future;
pub use tungstenite::protocol::{Message, Role};
pub use tungstenite::Error;

const PROTO_WEBSOCKET: &str = "websocket";
const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";
const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";

/// Check if a WebSocket upgrade was requested.
pub fn requested(headers: &HeaderMap) -> bool {
	headers.get(UPGRADE) == Some(&HeaderValue::from_static(PROTO_WEBSOCKET))
}

/// Accept a WebSocket upgrade request.
///
/// Returns HTTP response, and a future that eventually resolves
/// into websocket object.
pub fn accept(
	headers: &HeaderMap,
	body: Body,
) -> Result<
	(
		Response<Body>,
		impl Future<Output = Result<WebSocketStream<Upgraded>, gotham::hyper::Error>>,
	),
	(),
> {
	let res = response(headers)?;
	let ws = async {
		let upgraded = match body.on_upgrade().await {
			Ok(upgraded) => upgraded,
			Err(error) => {
				error!("Failed to upgrade connection: {}", error);
				return Err(error);
			}
		};

		const WEBSOCKET_CONFIGURATION: Option<WebSocketConfig> = Some(WebSocketConfig {
			max_send_queue: Some(1),
			max_message_size: Some(10 * 1024),
			max_frame_size: Some(10 * 1024),
		});
		Ok(WebSocketStream::from_raw_socket(upgraded, Role::Server, WEBSOCKET_CONFIGURATION).await)
	};

	Ok((res, ws))
}

fn response(headers: &HeaderMap) -> Result<Response<Body>, ()> {
	let key = headers.get(SEC_WEBSOCKET_KEY).ok_or(())?;

	Ok(Response::builder()
		.header(UPGRADE, PROTO_WEBSOCKET)
		.header(CONNECTION, "upgrade")
		.header(SEC_WEBSOCKET_ACCEPT, accept_key(key.as_bytes()))
		.status(StatusCode::SWITCHING_PROTOCOLS)
		.body(Body::empty())
		.unwrap())
}

fn accept_key(key: &[u8]) -> String {
	const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
	let mut sha1 = Sha1::default();
	sha1.update(key);
	sha1.update(WS_GUID);
	base64::encode(&sha1.digest().bytes())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn key_to_accept() {
		// From https://tools.ietf.org/html/rfc6455#section-1.2
		let key = accept_key("dGhlIHNhbXBsZSBub25jZQ==".as_bytes());
		assert_eq!(key, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
	}
}
