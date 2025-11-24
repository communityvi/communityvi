use crate::chat::repository::ChatRepository;
use crate::database::error::{DatabaseError, IntoStoreResult};
use crate::database::transaction::{DynTransaction, Transaction};
use crate::database::{Connection, Database, Repository};
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{Acquire, Sqlite, SqliteConnection, SqlitePool, SqliteTransaction, migrate};
use std::any::Any;
use std::ops::{Deref, DerefMut};

mod chat;
mod room;
#[cfg(test)]
pub mod test_utils;
mod user;

#[derive(Clone)]
pub struct SqliteDatabase {
	pool: SqlitePool,
}

impl SqliteDatabase {
	pub async fn connect(database_url: &str) -> Result<Self, DatabaseError> {
		let pool = SqlitePool::connect(database_url)
			.await
			.connection_error("Failed to connect to database")?;
		let store = Self { pool };

		Ok(store)
	}
}

#[async_trait]
impl Database for SqliteDatabase {
	async fn migrate(&mut self) -> Result<(), DatabaseError> {
		migrate!().run(&self.pool).await.map_err(Into::into)
	}

	async fn connection(&self) -> Result<Box<dyn Connection>, DatabaseError> {
		self.pool
			.acquire()
			.await
			.map(|connection| Box::new(connection) as Box<dyn Connection>)
			.map_err(Into::into)
	}
}

#[async_trait]
impl Connection for SqliteConnection {
	async fn begin_transaction<'connection>(&'connection mut self) -> Result<Transaction<'connection>, DatabaseError> {
		self.begin().await.map(Transaction::new).map_err(Into::into)
	}
}

#[async_trait]
impl Connection for PoolConnection<Sqlite> {
	async fn begin_transaction<'connection>(&'connection mut self) -> Result<Transaction<'connection>, DatabaseError> {
		// FIXME: Handle cancellation unsafety of the SQLX pool (if this is even an issue with SQLite)
		self.begin().await.map(Transaction::new).map_err(Into::into)
	}
}

#[async_trait]
impl<'connection> DynTransaction<'connection> for SqliteTransaction<'connection> {
	fn as_connection(&self) -> &dyn Connection {
		self.deref()
	}

	fn as_mut_connection(&mut self) -> &mut dyn Connection {
		self.as_mut()
	}

	async fn commit(self: Box<Self>) -> Result<(), DatabaseError> {
		self.commit().await.map_err(Into::into)
	}

	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError> {
		self.rollback().await.map_err(Into::into)
	}
}

#[derive(Default, Clone, Copy)]
pub struct SqliteRepository;

impl Repository for SqliteRepository {
	fn user(&self) -> &dyn UserRepository {
		self
	}

	fn room(&self) -> &dyn RoomRepository {
		self
	}

	fn chat(&self) -> &dyn ChatRepository {
		self
	}
}

fn sqlite_connection(connection: &mut dyn Connection) -> Result<&mut SqliteConnection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &mut dyn Any = connection;

	if connection.is::<PoolConnection<Sqlite>>() {
		return Ok(connection.downcast_mut::<PoolConnection<Sqlite>>().unwrap().deref_mut());
	}

	if connection.is::<SqliteConnection>() {
		return Ok(connection.downcast_mut::<SqliteConnection>().unwrap());
	}

	if connection.is::<SqliteTransaction>() {
		return Ok(connection.downcast_mut::<SqliteTransaction>().unwrap().deref_mut());
	}

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected SQLite connection, got {type_name}",
	)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::transaction::{ConnectionTransactionExtension, TransactionError};
	use sqlx::{Executor, query, query_scalar};
	use std::sync::Arc;

	#[tokio::test]
	async fn commits_transaction() {
		let database = database().await;
		let mut connection = database.connection().await.expect("Failed to get database connection");
		let test_number = 42;

		connection
			.run_in_transaction(move |transaction| async move {
				TestRepository
					.create(transaction.deref_mut(), test_number)
					.await
					.map_err(TransactionError::<()>::from)
			})
			.await
			.expect("Failed to write value in transaction");

		let number = TestRepository
			.get(connection.as_mut(), test_number)
			.await
			.expect("Failed to read row written in transaction");

		assert_eq!(Some(test_number), number);
	}

	struct TestRepository;

	impl TestRepository {
		async fn get(&self, connection: &mut dyn Connection, number: i32) -> Result<Option<i32>, DatabaseError> {
			let connection = sqlite_connection(connection)?;

			query_scalar("SELECT number FROM test WHERE number = ?1")
				.bind(number)
				.fetch_optional(connection)
				.await
				.map_err(Into::into)
		}

		async fn create(&self, connection: &mut dyn Connection, number: i32) -> Result<(), DatabaseError> {
			let connection = sqlite_connection(connection)?;

			query("INSERT INTO test (number) VALUES(?1)")
				.bind(number)
				.execute(connection)
				.await
				.map(drop)
				.map_err(Into::into)
		}
	}

	async fn database() -> Arc<dyn Database> {
		let database = SqliteDatabase::connect("sqlite::memory:")
			.await
			.expect("Failed to create in-memory SQLite database");

		database
			.pool
			.execute(TEST_SCHEMA)
			.await
			.expect("Failed to create test schema");

		Arc::new(database)
	}

	const TEST_SCHEMA: &str = "CREATE TABLE test (number INTEGER NOT NULL PRIMARY KEY);";
}
