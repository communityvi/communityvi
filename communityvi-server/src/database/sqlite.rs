use crate::database::error::{DatabaseError, IntoStoreResult};
use crate::database::{Connection, Database, Repository, TransactionOperation};
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqliteConnection, SqlitePool, migrate};
use std::any::Any;
use std::ops::DerefMut;

mod chat_message;
mod medium;
mod room;
#[cfg(test)]
mod test_utils;
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

	async fn run_in_transaction(&self, operation: &mut dyn TransactionOperation) -> Result<(), DatabaseError> {
		let mut transaction = self.pool.begin().await?;

		let result = operation.perform(&mut *transaction).await;
		if result.is_err() {
			transaction.rollback().await?;
		} else {
			transaction.commit().await?;
		}

		Ok(())
	}
}

impl Connection for SqliteConnection {}
impl Connection for PoolConnection<Sqlite> {}

#[derive(Default, Clone, Copy)]
pub struct SqliteRepository;

impl Repository for SqliteRepository {}

fn sqlite_connection(connection: &mut dyn Connection) -> Result<&mut SqliteConnection, DatabaseError> {
	let type_name = connection.type_name();

	let connection: &mut dyn Any = connection;

	if connection.is::<PoolConnection<Sqlite>>() {
		return Ok(connection.downcast_mut::<PoolConnection<Sqlite>>().unwrap().deref_mut());
	}

	if connection.is::<SqliteConnection>() {
		return Ok(connection.downcast_mut::<SqliteConnection>().unwrap());
	}

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected SQLite connection, got {type_name}",
	)))
}
