use std::sync::atomic::AtomicU64;

pub struct Room {
	pub offset: AtomicU64,
}
