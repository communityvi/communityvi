use crate::database::{Connection, Database};
use anyhow::anyhow;
use async_trait::async_trait;
use deadpool::managed::{Object, PoolError};
use std::any::Any;
use std::ops::DerefMut;
use turso::Error;

mod pool;

use crate::database::error::DatabaseError;
use crate::database::turso::pool::TursoManager;
pub use pool::TursoPool;

#[async_trait]
impl Database for TursoPool {
	async fn migrate(&mut self) -> Result<(), DatabaseError> {
		todo!()
	}

	async fn connection(&self) -> Result<Box<dyn Connection>, DatabaseError> {
		self.get()
			.await
			.map(|connection| Box::new(connection) as Box<dyn Connection>)
			.map_err(Into::into)
	}
}

impl Connection for Object<TursoManager> {}

impl From<PoolError<turso::Error>> for DatabaseError {
	fn from(pool_error: PoolError<Error>) -> Self {
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
		// TODO: Actually map these errors correctly, we need tests for that
		match error {
			ToSqlConversionFailure(_) => Self::Encode(error.into()),
			MutexError(_) => Self::Connection(error.into()),
			SqlExecutionFailure(_) => Self::Database(error.into()),
			WalOperationError(_) => Self::Database(error.into()),
			ConversionFailure(_) => Self::Decode(error.into()),
			QueryReturnedNoRows => Self::NotFound(error.into()),
		}
	}
}

fn turso_connection(connection: &mut dyn Connection) -> Result<&mut turso::Connection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &mut dyn Any = connection;
	connection
		.downcast_mut::<Object<TursoManager>>()
		.map(|object| object.deref_mut())
		.ok_or_else(|| DatabaseError::DatabaseMismatch(anyhow!("Expected Turso connection, got {type_name}")))
}
