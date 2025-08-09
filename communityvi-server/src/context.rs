use crate::configuration::Configuration;
use crate::reference_time::ReferenceTimer;
use crate::store::sqlite::SqliteStore;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub store: SqliteStore,
}

impl ApplicationContext {
	pub async fn new(configuration: Configuration, time_source: TimeSource) -> anyhow::Result<ApplicationContext> {
		let reference_timer = ReferenceTimer::default();

		let store = SqliteStore::new("sqlite::memory:").await?;

		Ok(Self {
			configuration,
			time_source,
			reference_timer,
			store,
		})
	}
}
