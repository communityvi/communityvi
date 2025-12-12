use crate::database::sqlite::{SqliteDatabase, SqliteRepository};
use crate::database::{Connection, Database, Repository, TestFactory};
use std::sync::Arc;

pub struct SqliteTestFactory;

impl TestFactory for SqliteTestFactory {
	async fn connection() -> Box<dyn Connection> {
		connection().await
	}

	fn repository() -> Box<dyn Repository> {
		Box::new(SqliteRepository)
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
	let mut database = SqliteDatabase::connect("sqlite::memory:")
		.await
		.expect("Failed to create in-memory SQLite database");
	database.migrate().await.expect("Failed to migrate database");

	Arc::new(database)
}

pub fn repository() -> Arc<dyn Repository> {
	Arc::new(SqliteRepository)
}
