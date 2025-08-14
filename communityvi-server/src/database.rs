use crate::chat_message::repository::ChatMessageRepository;
use crate::database::error::DatabaseError;
use crate::room::medium::repository::MediumRepository;
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;
use std::any::{Any, type_name};

pub mod sqlite;

pub mod error;

#[async_trait]
pub trait Database: Send + Sync {
	async fn migrate(&mut self) -> Result<(), DatabaseError>;

	async fn connection(&self) -> Result<Box<dyn Connection>, DatabaseError>;
	async fn run_in_transaction(&self, transacton: &mut dyn TransactionOperation) -> Result<(), DatabaseError>;
}

assert_obj_safe!(Database);

pub trait Connection: Any + Send {
	fn type_name(&self) -> &'static str {
		type_name::<Self>()
	}
}

assert_obj_safe!(Connection);

pub trait Repository: UserRepository + RoomRepository + MediumRepository + ChatMessageRepository + Send + Sync {
	fn user(&self) -> &dyn UserRepository
	where
		Self: Sized,
	{
		self
	}

	fn room(&self) -> &dyn RoomRepository
	where
		Self: Sized,
	{
		self
	}

	fn medium(&self) -> &dyn MediumRepository
	where
		Self: Sized,
	{
		self
	}

	fn chat_message(&self) -> &dyn ChatMessageRepository
	where
		Self: Sized,
	{
		self
	}
}

assert_obj_safe!(Repository);

#[async_trait]
pub trait TransactionOperation: Send {
	async fn perform(&mut self, connection: &mut dyn Connection) -> Result<(), DatabaseError>;
}

assert_obj_safe!(TransactionOperation);
