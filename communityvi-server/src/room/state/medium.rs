use crate::room::state::medium::fixed_length::FixedLengthMedium;
use crate::room::state::medium::playback_state::PlaybackState;
use chrono::Duration;

pub mod fixed_length;
pub mod playback_state;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Medium {
	Empty,
	FixedLength(FixedLengthMedium),
}

impl Default for Medium {
	fn default() -> Self {
		Medium::Empty
	}
}

impl Medium {
	pub fn playback_state(&self) -> PlaybackState {
		match self {
			Medium::Empty => PlaybackState::Paused {
				at_position: Duration::milliseconds(0),
			},
			Medium::FixedLength(medium) => medium.playback,
		}
	}

	pub fn play(&mut self, start_time: Duration, reference_now: Duration) {
		match self {
			Medium::Empty => {}
			Medium::FixedLength(medium) => medium.play(start_time, reference_now),
		}
	}

	pub fn pause(&mut self, at_position: Duration) {
		match self {
			Medium::Empty => {}
			Medium::FixedLength(medium) => medium.pause(at_position),
		}
	}
}

impl From<FixedLengthMedium> for Medium {
	fn from(fixed_length_medium: FixedLengthMedium) -> Self {
		Medium::FixedLength(fixed_length_medium)
	}
}
