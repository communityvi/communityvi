use crate::database::libsql::pool::LibSqlManager;
use crate::database::libsql::{LibSqlPool, LibSqlRepository};
use crate::database::{Connection, Database, Repository, TestFactory};
use std::sync::Arc;

pub struct LibSqlTestFactory;

impl TestFactory for LibSqlTestFactory {
	async fn connection() -> Box<dyn Connection> {
		connection().await
	}

	fn repository() -> Box<dyn Repository> {
		Box::new(LibSqlRepository)
	}
}

pub async fn connection() -> Box<dyn Connection> {
	database()
		.await
		.connection()
		.await
		.expect("Failed to connect to database")
}

pub async fn database() -> Arc<dyn Database> {
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
