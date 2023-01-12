use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RoomError {
	#[error("Can't join, room is already full.")]
	RoomFull,
}
