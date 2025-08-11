use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::user::model;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository {
	async fn get(&self, connection: &mut dyn Connection, user_uuid: Uuid)
	-> Result<Option<model::User>, DatabaseError>;
	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<model::User, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<(), DatabaseError>;
}

assert_obj_safe!(UserRepository);
