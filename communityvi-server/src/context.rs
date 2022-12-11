use crate::configuration::Configuration;
use crate::reference_time::ReferenceTimer;
use async_time_mock_tokio::MockableClock;

#[derive(Clone)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub clock: MockableClock,
	pub reference_timer: ReferenceTimer,
}

impl ApplicationContext {
	pub fn new(configuration: Configuration, clock: MockableClock) -> ApplicationContext {
		Self {
			configuration,
			clock,
			reference_timer: ReferenceTimer::default(),
		}
	}
}
