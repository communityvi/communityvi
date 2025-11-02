use crate::database::error::DatabaseError;
use crate::user::UserCreationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoomError {
	#[error("Name was empty or whitespace-only.")]
	EmptyClientName,
	#[error("Client name is already in use.")]
	ClientNameAlreadyInUse,
	#[error("Client name is too long. (>256 bytes UTF-8)")]
	ClientNameTooLong,
	#[error("Can't join, room is already full.")]
	RoomFull,
	#[error("Database error: {0}")]
	Database(#[from] DatabaseError),
}

impl From<UserCreationError> for RoomError {
	fn from(creation_error: UserCreationError) -> Self {
		use UserCreationError::*;

		match creation_error {
			NameEmpty => Self::EmptyClientName,
			NameTooLong => Self::ClientNameTooLong,
			NameAlreadyInUse => Self::ClientNameAlreadyInUse,
			Database(error) => Self::Database(error),
		}
	}
}
