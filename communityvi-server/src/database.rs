use crate::database::error::DatabaseError;
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
}

assert_obj_safe!(Database);

pub trait Connection: Any + Send {
	fn type_name(&self) -> &'static str {
		type_name::<Self>()
	}
}

assert_obj_safe!(Connection);

pub trait Repository: UserRepository + Send + Sync {
	fn user(&self) -> &dyn UserRepository
	where
		Self: Sized,
	{
		self
	}
}

assert_obj_safe!(Repository);
