use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model;
use async_trait::async_trait;
use static_assertions::assert_obj_safe;
use uuid::Uuid;

#[async_trait]
pub trait RoomRepository {
	async fn get(&self, connection: &mut dyn Connection, room_uuid: Uuid)
	-> Result<Option<model::Room>, DatabaseError>;
	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<model::Room, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<(), DatabaseError>;
}

assert_obj_safe!(RoomRepository);
