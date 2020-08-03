use crate::configuration::Configuration;
use crate::utils::time_source::TimeSource;

#[derive(Clone)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
}

impl ApplicationContext {
	pub fn new(configuration: Configuration, time_source: TimeSource) -> ApplicationContext {
		Self {
			configuration,
			time_source,
		}
	}
}
