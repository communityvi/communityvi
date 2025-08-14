use sqlx::FromRow;
use uuid::Uuid;

#[derive(FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Medium {
	pub uuid: Uuid,
	pub name: String,
	pub version: i64,
	pub length_ms: i64,
	pub playback_state: PlaybackState,
	pub playback_state_start_time_ms: Option<i64>,
	pub playback_state_at_position_ms: Option<i64>,
}

#[derive(sqlx::Type, Clone, Copy, Debug, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
pub enum PlaybackState {
	Playing,
	Paused,
}
