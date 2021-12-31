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
