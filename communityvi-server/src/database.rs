use crate::chat::repository::ChatRepository;
use crate::database::error::DatabaseError;
use crate::room::repository::RoomRepository;
use crate::user::repository::UserRepository;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;
use std::any::{Any, type_name};

pub mod libsql;

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

pub trait Repository: UserRepository + RoomRepository + ChatRepository + Send + Sync + 'static {
	fn user(&self) -> &dyn UserRepository;
	fn room(&self) -> &dyn RoomRepository;
	fn chat(&self) -> &dyn ChatRepository;
}

assert_obj_safe!(Repository);

#[cfg(test)]
pub(crate) mod test {
	use crate::database::{Connection, Database, Repository};
	use std::sync::Arc;

	pub type DefaultTestFactory = crate::database::libsql::test_utils::LibSqlTestFactory;

	pub trait TestFactory {
		async fn connection() -> Box<dyn Connection>;
		async fn database() -> Arc<dyn Database>;
		fn repository() -> Arc<dyn Repository>;
	}
}
