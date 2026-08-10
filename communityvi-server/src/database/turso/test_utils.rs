use crate::database::test::TestFactory;
use crate::database::turso::pool::TursoManager;
use crate::database::turso::{TursoPool, TursoRepository};
use crate::database::{Connection, Database, Repository};
use std::sync::Arc;

pub struct TursoTestFactory;

impl TestFactory for TursoTestFactory {
	async fn connection() -> Box<dyn Connection> {
		Self::database()
			.await
			.connection()
			.await
			.expect("Failed to connect to database")
	}

	async fn database() -> Arc<dyn Database> {
		let database = turso::Builder::new_local(":memory:")
			.build()
			.await
			.expect("Failed to build turso database");
		let manager = TursoManager::new(database);
		let mut pool = TursoPool::builder(manager).build().expect("Failed to build turso pool");

		pool.migrate().await.expect("Failed to migrate database");

		Arc::new(pool)
	}

	fn repository() -> Arc<dyn Repository> {
		Arc::new(TursoRepository)
	}
}
