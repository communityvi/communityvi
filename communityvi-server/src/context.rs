use crate::configuration::Configuration;
use crate::reference_time::ReferenceTimer;
use crate::user::UserRepository;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub user_repository: Arc<Mutex<UserRepository>>,
}

impl ApplicationContext {
	pub fn new(configuration: Configuration, time_source: TimeSource) -> ApplicationContext {
		Self {
			configuration,
			time_source,
			reference_timer: Default::default(),
			user_repository: Default::default(),
		}
	}
}
