use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;

#[async_trait]
pub trait RoomRepository: Send + Sync + 'static {
	async fn get(&self, connection: &mut dyn Connection, room_uuid: Uuid)
	-> Result<Option<model::Room>, DatabaseError>;
	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<model::Room, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<(), DatabaseError>;
	async fn get_all_users(&self, connection: &mut dyn Connection, room_uuid: Uuid)
	-> Result<Vec<User>, DatabaseError>;
	async fn add_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError>;
	async fn remove_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError>;
}

assert_obj_safe!(RoomRepository);
