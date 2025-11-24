use crate::chat::repository::ChatRepository;
use crate::database::error::DatabaseError;
use crate::database::transaction::Transaction;
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;
use std::any::{Any, type_name};
use std::ops::DerefMut;

pub mod sqlite;

pub mod error;
pub mod transaction;

#[async_trait]
pub trait Database: Send + Sync {
	async fn migrate(&mut self) -> Result<(), DatabaseError>;

	async fn connection(&self) -> Result<Box<dyn Connection>, DatabaseError>;
}

assert_obj_safe!(Database);

#[async_trait]
pub trait Connection: Any + Send {
	fn type_name(&self) -> &'static str {
		type_name::<Self>()
	}

	async fn begin_transaction<'connection>(&'connection mut self) -> Result<Transaction<'connection>, DatabaseError>;
}

#[async_trait]
impl Connection for Box<dyn Connection> {
	async fn begin_transaction<'connection>(&'connection mut self) -> Result<Transaction<'connection>, DatabaseError> {
		self.deref_mut().begin_transaction().await
	}
}

assert_obj_safe!(Connection);

pub trait Repository: UserRepository + RoomRepository + ChatRepository + Send + Sync + 'static {
	fn user(&self) -> &dyn UserRepository;
	fn room(&self) -> &dyn RoomRepository;
	fn chat(&self) -> &dyn ChatRepository;
}

assert_obj_safe!(Repository);
