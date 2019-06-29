use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct State {
	pub offset: Arc<AtomicU64>,
}
