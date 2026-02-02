use crate::database::{Connection, Database, Repository};
use anyhow::{Context, anyhow};
use async_trait::async_trait;
use deadpool::managed::{Object, PoolError};
use std::any::Any;
use std::ops::Deref;

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

impl Connection for Object<LibSqlManager> {}

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
	connection
		.downcast_ref::<Object<LibSqlManager>>()
		.map(Deref::deref)
		.ok_or_else(|| DatabaseError::DatabaseMismatch(anyhow!("Expected LibSql connection, got {type_name}")))
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
