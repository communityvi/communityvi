use crate::configuration::Configuration;
use crate::reference_time::ReferenceTimer;
use crate::utils::time_source::TimeSource;

#[derive(Clone)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
}

impl ApplicationContext {
	pub fn new(configuration: Configuration, time_source: TimeSource) -> ApplicationContext {
		Self {
			configuration,
			time_source,
			reference_timer: ReferenceTimer::default(),
		}
	}
}
