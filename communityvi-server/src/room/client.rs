use crate::connection::client::ClientConnection;

pub struct Client {
	pub name: String,
	pub connection: ClientConnection,
}
