use crate::database::libsql::pool::LibSqlManager;
use crate::database::libsql::{LibSqlPool, LibSqlRepository};
use crate::database::test::TestFactory;
use crate::database::{Connection, Database, Repository};
use std::sync::Arc;

pub struct LibSqlTestFactory;

impl TestFactory for LibSqlTestFactory {
	async fn connection() -> Box<dyn Connection> {
		Self::database()
			.await
			.connection()
			.await
			.expect("Failed to connect to database")
	}

	async fn database() -> Arc<dyn Database> {
		let database = libsql::Builder::new_local(":memory:")
			.build()
			.await
			.expect("Failed to build libsql database");
		let manager = LibSqlManager::new(database);
		let mut pool = LibSqlPool::builder(manager)
			.build()
			.expect("Failed to build libsql pool");

		pool.migrate().await.expect("Failed to migrate database");

		Arc::new(pool)
	}

	fn repository() -> Arc<dyn Repository> {
		Arc::new(LibSqlRepository)
	}
}
