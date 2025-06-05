use crate::configuration::Configuration;
use crate::reference_time::ReferenceTimer;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;
use sqlx::SqlitePool;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub database_pool: SqlitePool,
}

impl ApplicationContext {
	pub async fn new(configuration: Configuration, time_source: TimeSource) -> anyhow::Result<ApplicationContext> {
		let reference_timer = ReferenceTimer::default();

		let database_pool = SqlitePool::connect("sqlite::memory:").await?;
		sqlx::migrate!("./migrations").run(&database_pool).await?;

		Ok(Self {
			configuration,
			time_source,
			reference_timer,
			database_pool,
		})
	}
}
