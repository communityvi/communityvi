use std::sync::atomic::AtomicU64;

pub struct State {
	pub offset: AtomicU64,
}
