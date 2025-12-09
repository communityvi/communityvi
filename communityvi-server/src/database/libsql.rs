use crate::database::{Connection, Database};
use anyhow::anyhow;
use async_trait::async_trait;
use deadpool::managed::{Object, PoolError};
use std::any::Any;
use std::ops::DerefMut;

mod migration;
mod pool;
#[cfg(test)]
mod test_utils;

use crate::database::error::DatabaseError;
use crate::database::libsql::pool::LibSqlManager;
pub use pool::LibSqlPool;

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
		// TODO: Actually map these errors correctly, we need tests for that
		match error {
			ToSqlConversionFailure(_) => Self::Encode(error.into()),
			QueryReturnedNoRows => Self::NotFound(error.into()),
			InvalidColumnIndex | InvalidColumnType => Self::Decode(error.into()),
			ConnectionFailed(_) | InvalidUTF8Path | InvalidParserState(_) | InvalidTlsConfiguration(_) => {
				Self::Connection(error.into())
			}
			_ => Self::Database(error.into()),
		}
	}
}

fn libsql_connection(connection: &mut dyn Connection) -> Result<&mut libsql::Connection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &mut dyn Any = connection;
	connection
		.downcast_mut::<Object<LibSqlManager>>()
		.map(DerefMut::deref_mut)
		.ok_or_else(|| DatabaseError::DatabaseMismatch(anyhow!("Expected LibSql connection, got {type_name}")))
}
