use crate::database::sqlite::SqliteDatabase;
use crate::database::{Connection, Database};

pub async fn connection() -> Box<dyn Connection> {
	let mut database = SqliteDatabase::connect("sqlite::memory:")
		.await
		.expect("Failed to create in-memory SQLite database");
	database.migrate().await.expect("Failed to migrate database");

	database.connection().await.expect("Failed to connect to database")
}
