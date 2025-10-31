use crate::chat::repository::ChatRepository;
use crate::database::error::{DatabaseError, IntoStoreResult};
use crate::database::{Connection, Database, Repository};
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqliteConnection, SqlitePool, migrate};
use std::any::Any;
use std::ops::DerefMut;

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

impl Connection for SqliteConnection {}
impl Connection for PoolConnection<Sqlite> {}

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

	Err(DatabaseError::DatabaseMismatch(anyhow!(
		"Expected SQLite connection, got {type_name}",
	)))
}
