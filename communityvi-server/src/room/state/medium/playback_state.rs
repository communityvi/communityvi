use chrono::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
	/// Reference time when the medium would need to have started playing.
	/// (Relative to reference time)
	Playing { start_time: Duration },
	/// Position in the medium where it is paused.
	/// (Relative to start of medium)
	Paused { at_position: Duration },
}

impl Default for PlaybackState {
	fn default() -> Self {
		Self::Paused {
			at_position: Duration::seconds(0),
		}
	}
}
