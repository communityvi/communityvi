use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub enum PlaybackState {
	/// Currently playing a video that would have started playing at the given point in reference time.
	Playing { start: Instant },
	/// Video is currently paused at the given position in the video.
	Paused { position: Duration },
	/// No video loaded
	Empty,
}

impl Default for PlaybackState {
	fn default() -> Self {
		PlaybackState::Empty
	}
}
