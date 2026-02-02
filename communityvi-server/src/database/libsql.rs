use crate::database::{Connection, Database, Repository};
use anyhow::{Context, anyhow};
use async_trait::async_trait;
use deadpool::managed::{Object, PoolError};
use std::any::Any;
use std::ops::{Deref, DerefMut};

mod chat;
mod migration;
mod pool;
mod room;
#[cfg(test)]
pub mod test_utils;
mod user;

use crate::chat::repository::ChatRepository;
use crate::database::error::DatabaseError;
use crate::database::libsql::pool::LibSqlManager;
use crate::database::transaction::Transaction;
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
pub use pool::LibSqlPool;

pub async fn create_pool(path: impl AsRef<std::path::Path>) -> anyhow::Result<LibSqlPool> {
	let database = libsql::Builder::new_local(path)
		.build()
		.await
		.context("Failed to build libsql database")?;
	let manager = LibSqlManager::new(database);

	LibSqlPool::builder(manager)
		.build()
		.context("Failed to build libsql pool")
}

#[async_trait]
impl Database for LibSqlPool {
	async fn migrate(&mut self) -> Result<(), DatabaseError> {
		let connection = self.connection().await?;
		migration::run_migrations(connection.as_ref()).await?;

		Ok(())
	}

	async fn connection(&self) -> Result<Box<dyn Connection>, DatabaseError> {
		self.get()
			.await
			.map(|connection| Box::new(connection) as Box<dyn Connection>)
			.map_err(Into::into)
	}
}

#[async_trait]
impl Connection for Object<LibSqlManager> {
	async fn begin_transaction<'connection>(
		&'connection mut self,
	) -> Result<Box<dyn Transaction<'connection>>, DatabaseError> {
		self.deref_mut().begin_transaction().await
	}
}

#[async_trait]
impl Connection for libsql::Connection {
	async fn begin_transaction<'connection>(
		&'connection mut self,
	) -> Result<Box<dyn Transaction<'connection>>, DatabaseError> {
		// TODO: Figure out which transaction behavior to use, default is DEFERRED.
		self.transaction()
			.await
			.map(|transaction| Box::new(transaction) as Box<dyn Transaction<'connection>>)
			.map_err(Into::into)
	}
}

impl From<PoolError<libsql::Error>> for DatabaseError {
	fn from(pool_error: PoolError<libsql::Error>) -> Self {
		use PoolError::*;
		match pool_error {
			Timeout(_) => Self::Timeout(pool_error.into()),
			Backend(error) => error.into(),
			Closed | NoRuntimeSpecified | PostCreateHook(_) => Self::Connection(pool_error.into()),
		}
	}
}

impl From<libsql::Error> for DatabaseError {
	fn from(error: libsql::Error) -> Self {
		use libsql::Error::*;
		match error {
			ToSqlConversionFailure(_) => Self::Encode(error.into()),
			QueryReturnedNoRows => Self::NotFound(error.into()),
			InvalidColumnIndex | InvalidColumnType => Self::Decode(error.into()),
			ConnectionFailed(_) | InvalidUTF8Path | InvalidParserState(_) | InvalidTlsConfiguration(_) => {
				Self::Connection(error.into())
			}
			// https://sqlite.org/rescode.html
			SqliteFailure(code, message) if [2067, 1555].contains(&code) => Self::UniqueViolation(anyhow!("{message}")),
			SqliteFailure(787, message) => Self::ForeignKeyViolation(anyhow!("{message}")),
			SqliteFailure(code, message) if [275, 531, 3091, 1043, 1299, 2835, 2579, 1811].contains(&code) => {
				Self::OtherConstraintViolation(anyhow!("{message}"))
			}
			SqliteFailure(773, message) => Self::Timeout(anyhow!("{message}")),
			SqliteFailure(3338, message) => Self::Connection(anyhow!("{message}")),
			_ => Self::Database(dbg!(error).into()),
		}
	}
}

fn libsql_connection(connection: &dyn Connection) -> Result<&libsql::Connection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &dyn Any = connection;
	if let Some(connection) = connection.downcast_ref::<Object<LibSqlManager>>().map(Deref::deref) {
		return Ok(connection);
	}
	if let Some(connection) = connection.downcast_ref::<libsql::Connection>() {
		return Ok(connection);
	}

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected LibSql connection, got {type_name}"
	)))
}

#[derive(Default, Clone, Copy)]
pub struct LibSqlRepository;

impl Repository for LibSqlRepository {
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

#[async_trait]
impl Transaction<'_> for libsql::Transaction {
	fn as_connection(&self) -> &dyn Connection {
		self.deref()
	}

	async fn commit(self: Box<Self>) -> Result<(), DatabaseError> {
		self.commit().await.map_err(Into::into)
	}

	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError> {
		self.rollback().await.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Database;
	use crate::database::libsql::test_utils::LibSqlTestFactory;
	use crate::database::test::TestFactory;
	use crate::database::transaction::{ConnectionTransactionExtension, TransactionError};
	use std::sync::Arc;

	#[tokio::test]
	async fn commits_transaction() {
		let database = database().await;
		let connection = database.connection().await.expect("Failed to get database connection");
		let test_number = 42;

		connection
			.run_in_transaction(async move |transaction| {
				TestRepository
					.create(transaction.as_connection(), test_number)
					.await
					.map_err(TransactionError::<()>::from)
			})
			.await
			.expect("Failed to write value in transaction");

		let number = TestRepository
			.get(connection.as_ref(), test_number)
			.await
			.expect("Failed to read row written in transaction");

		assert_eq!(Some(test_number), number);
	}

	struct TestRepository;

	impl TestRepository {
		async fn get(&self, connection: &dyn Connection, number: i32) -> Result<Option<i32>, DatabaseError> {
			let connection = libsql_connection(connection)?;

			let mut rows = connection
				.query("SELECT number FROM test WHERE number = ?1", [number])
				.await?;

			let Some(row) = rows.next().await? else {
				return Ok(None);
			};

			Ok(Some(
				row.get(0).map_err(anyhow::Error::from).map_err(DatabaseError::Decode)?,
			))
		}

		async fn create(&self, connection: &dyn Connection, number: i32) -> Result<(), DatabaseError> {
			let connection = libsql_connection(connection)?;

			connection
				.execute("INSERT INTO test (number) VALUES(?1)", [number])
				.await
				.map(drop)
				.map_err(Into::into)
		}
	}

	async fn database() -> Arc<dyn Database> {
		let database = LibSqlTestFactory::database().await;
		let connection = database.connection().await.expect("Failed to get database connection");
		let connection = libsql_connection(connection.as_ref()).expect("Failed to get concrete database connection");

		connection
			.execute_batch(TEST_SCHEMA)
			.await
			.expect("Failed to create test schema");

		database
	}

	const TEST_SCHEMA: &str = "CREATE TABLE test (number INTEGER NOT NULL PRIMARY KEY);";
}
