use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum RoomError {
	EmptyClientName,
	ClientNameAlreadyInUse,
	ClientNameTooLong,
	RoomFull,
}

impl Display for RoomError {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		match self {
			RoomError::EmptyClientName => formatter.write_str("Name was empty or whitespace-only."),
			RoomError::ClientNameAlreadyInUse => formatter.write_str("Client name is already in use."),
			RoomError::ClientNameTooLong => formatter.write_str("Client name is too long. (>256 bytes UTF-8)"),
			RoomError::RoomFull => formatter.write_str("Can't join, room is already full."),
		}
	}
}

impl Error for RoomError {}
