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
use crate::database::transaction::Transaction;
use crate::database::turso::pool::TursoManager;
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
pub use pool::TursoPool;

pub async fn create_pool(path: impl AsRef<std::path::Path>) -> anyhow::Result<TursoPool> {
	let path = path.as_ref().to_str().context("Database path is not valid UTF-8")?;
	let database = turso::Builder::new_local(path)
		.build()
		.await
		.context("Failed to build turso database")?;
	let manager = TursoManager::new(database);

	TursoPool::builder(manager)
		.build()
		.context("Failed to build turso pool")
}

#[async_trait]
impl Database for TursoPool {
	async fn migrate(&mut self) -> Result<(), DatabaseError> {
		let mut connection = self.connection().await?;
		migration::run_migrations(connection.as_mut()).await?;

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
impl Connection for Object<TursoManager> {
	async fn begin_transaction<'connection>(
		&'connection mut self,
	) -> Result<Box<dyn Transaction + 'connection>, DatabaseError> {
		self.deref_mut().begin_transaction().await
	}
}

#[async_trait]
impl Connection for turso::Connection {
	async fn begin_transaction<'connection>(
		&'connection mut self,
	) -> Result<Box<dyn Transaction + 'connection>, DatabaseError> {
		// TODO: Figure out which transaction behavior to use, default is DEFERRED.
		self.transaction()
			.await
			.map(|transaction| Box::new(transaction) as Box<dyn Transaction + 'connection>)
			.map_err(Into::into)
	}
}

impl From<PoolError<turso::Error>> for DatabaseError {
	fn from(pool_error: PoolError<turso::Error>) -> Self {
		use PoolError::*;
		match pool_error {
			Timeout(_) => Self::Timeout(pool_error.into()),
			Backend(error) => error.into(),
			Closed | NoRuntimeSpecified | PostCreateHook(_) => Self::Connection(pool_error.into()),
		}
	}
}

impl From<turso::Error> for DatabaseError {
	fn from(error: turso::Error) -> Self {
		use turso::Error::*;
		match error {
			ToSqlConversionFailure(_) => Self::Encode(error.into()),
			QueryReturnedNoRows => Self::NotFound(error.into()),
			ConversionFailure(_) => Self::Decode(error.into()),
			Busy(_) => Self::Timeout(error.into()),
			BusySnapshot(_) => Self::TransactionSerialization(error.into()),
			Constraint(ref message) => classify_constraint_violation(&message.clone(), error),
			IoError(..) => Self::Connection(error.into()),
			Corrupt(_) | NotAdb(_) | Error(_) | Misuse(_) | Interrupt(_) | Readonly(_) | DatabaseFull(_) => {
				Self::Database(error.into())
			}
		}
	}
}

fn classify_constraint_violation(message: &str, error: turso::Error) -> DatabaseError {
	if message.contains("UNIQUE constraint failed") || message.contains("PRIMARY KEY constraint failed") {
		DatabaseError::UniqueViolation(error.into())
	} else if message.contains("FOREIGN KEY constraint failed") {
		DatabaseError::ForeignKeyViolation(error.into())
	} else {
		DatabaseError::OtherConstraintViolation(error.into())
	}
}

fn turso_connection(connection: &dyn Connection) -> Result<&turso::Connection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &dyn Any = connection;
	if let Some(connection) = connection.downcast_ref::<Object<TursoManager>>().map(Deref::deref) {
		return Ok(connection);
	}
	if let Some(connection) = connection.downcast_ref::<turso::Connection>() {
		return Ok(connection);
	}

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected Turso connection, got {type_name}"
	)))
}

fn turso_connection_mut(connection: &mut dyn Connection) -> Result<&mut turso::Connection, DatabaseError> {
	let type_name = connection.type_name();

	// `downcast_mut` can't be chained directly: a failed first attempt would still hold
	// `connection` mutably borrowed for the second, so check the type with `is` first.
	let connection: &mut dyn Any = connection;
	if connection.is::<Object<TursoManager>>() {
		return Ok(connection
			.downcast_mut::<Object<TursoManager>>()
			.unwrap_or_else(|| unreachable!("just checked with `is`"))
			.deref_mut());
	}
	if connection.is::<turso::Connection>() {
		return Ok(connection
			.downcast_mut::<turso::Connection>()
			.unwrap_or_else(|| unreachable!("just checked with `is`")));
	}

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected Turso connection, got {type_name}"
	)))
}

#[derive(Default, Clone, Copy)]
pub struct TursoRepository;

impl Repository for TursoRepository {
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
impl Transaction for turso::transaction::Transaction<'_> {
	fn as_connection(&self) -> &dyn Connection {
		self.deref()
	}

	async fn commit(self: Box<Self>) -> Result<(), DatabaseError> {
		(*self).commit().await.map_err(Into::into)
	}

	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError> {
		(*self).rollback().await.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Database;
	use crate::database::test::TestFactory;
	use crate::database::transaction::{ConnectionTransactionExtension, TransactionError};
	use crate::database::turso::test_utils::TursoTestFactory;
	use std::sync::Arc;

	#[tokio::test]
	async fn commits_transaction() {
		let database = database().await;
		let mut connection = database.connection().await.expect("Failed to get database connection");
		let test_number = 42;

		connection
			.run_in_transaction(|transaction| {
				Box::pin(async move {
					TestRepository
						.create(transaction.as_connection(), test_number)
						.await
						.map_err(TransactionError::<()>::from)
				})
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
			let connection = turso_connection(connection)?;

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
			let connection = turso_connection(connection)?;

			connection
				.execute("INSERT INTO test (number) VALUES(?1)", [number])
				.await
				.map(drop)
				.map_err(Into::into)
		}
	}

	async fn database() -> Arc<dyn Database> {
		let database = TursoTestFactory::database().await;
		let connection = database.connection().await.expect("Failed to get database connection");
		let connection = turso_connection(connection.as_ref()).expect("Failed to get concrete database connection");

		connection
			.execute_batch(TEST_SCHEMA)
			.await
			.expect("Failed to create test schema");

		database
	}

	const TEST_SCHEMA: &str = "CREATE TABLE test (number INTEGER NOT NULL PRIMARY KEY);";
}
