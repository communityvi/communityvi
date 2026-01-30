use crate::configuration::Configuration;
use crate::database::libsql::{LibSqlRepository, create_pool};
use crate::database::{Database, Repository};
use crate::reference_time::ReferenceTimer;
use crate::user::UserService;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;
use std::sync::Arc;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub user_service: UserService,
	pub database: Arc<dyn Database>,
	pub repository: Arc<dyn Repository>,
}

impl ApplicationContext {
	pub async fn new(configuration: Configuration, time_source: TimeSource) -> anyhow::Result<ApplicationContext> {
		let reference_timer = ReferenceTimer::default();

		let mut pool = create_pool(":memory:").await?;
		pool.migrate().await?;

		let database = Arc::new(pool);
		let repository = Arc::new(LibSqlRepository);

		let user_service = UserService::new(repository.clone());

		Ok(Self {
			configuration,
			time_source,
			reference_timer,
			user_service,
			database,
			repository,
		})
	}
}
