use crate::user::UserCreationError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RoomError {
	#[error("Name was empty or whitespace-only.")]
	EmptyClientName,
	#[error("Client name is already in use.")]
	ClientNameAlreadyInUse,
	#[error("Client name is too long. (>256 bytes UTF-8)")]
	ClientNameTooLong,
	#[error("Can't join, room is already full.")]
	RoomFull,
}

impl From<UserCreationError> for RoomError {
	fn from(creation_error: UserCreationError) -> Self {
		use UserCreationError::*;

		match creation_error {
			NameEmpty => Self::EmptyClientName,
			NameTooLong => Self::ClientNameTooLong,
			NameAlreadyInUse => Self::ClientNameAlreadyInUse,
		}
	}
}
