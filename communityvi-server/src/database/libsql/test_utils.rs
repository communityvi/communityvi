use crate::database::Database;
use crate::database::libsql::LibSqlPool;
use crate::database::libsql::pool::LibSqlManager;
use std::sync::Arc;

pub async fn connection() -> Box<dyn crate::database::Connection> {
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
		.expect("Failed to build turso database");
	let manager = LibSqlManager::new(database);
	let mut pool = LibSqlPool::builder(manager)
		.build()
		.expect("Failed to build turso pool");

	pool.migrate().await.expect("Failed to migrate database");

	Arc::new(pool)
}
