use crate::connection::client::ClientConnection;
use debug_stub_derive::DebugStub;

#[derive(DebugStub)]
pub struct Client {
	pub name: String,
	#[debug_stub = "ClientConnection"]
	pub connection: ClientConnection,
}
