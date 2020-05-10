use crate::room::state::medium::fixed_length::FixedLengthMedium;
use crate::room::state::medium::playback_state::PlaybackState;
use chrono::Duration;
use std::ops::{Deref, DerefMut};

pub mod fixed_length;
pub mod playback_state;

pub trait Medium {
	/// Start playing such that the beginning of the medium would be at the given reference time.
	fn play(&mut self, start_time: Duration, reference_now: Duration) -> PlaybackState;
	/// Pause at the given position in the medium.
	fn pause(&mut self, at_position: Duration) -> PlaybackState;
	/// Query the current `PlaybackState`
	fn playback_state(&self) -> PlaybackState;
	/// Name of the medium
	fn name(&self) -> &str;
}

#[derive(Clone, Debug)]
pub enum SomeMedium {
	FixedLength(FixedLengthMedium),
}

impl Deref for SomeMedium {
	type Target = dyn Medium;

	fn deref(&self) -> &Self::Target {
		match self {
			SomeMedium::FixedLength(fixed_length_medium) => fixed_length_medium,
		}
	}
}

impl DerefMut for SomeMedium {
	fn deref_mut(&mut self) -> &mut Self::Target {
		match self {
			SomeMedium::FixedLength(fixed_length_medium) => fixed_length_medium,
		}
	}
}
