use crate::configuration::Configuration;
use crate::database::sqlite::{SqliteDatabase, SqliteRepository};
use crate::database::{Database, Repository};
use crate::reference_time::ReferenceTimer;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;
use std::sync::Arc;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub database: Arc<dyn Database>,
	pub repository: Arc<dyn Repository>,
}

impl ApplicationContext {
	pub async fn new(configuration: Configuration, time_source: TimeSource) -> anyhow::Result<ApplicationContext> {
		let reference_timer = ReferenceTimer::default();

		let database = Arc::new(SqliteDatabase::connect("sqlite::memory:").await?);
		let repository = Arc::new(SqliteRepository);

		Ok(Self {
			configuration,
			time_source,
			reference_timer,
			database,
			repository,
		})
	}
}
