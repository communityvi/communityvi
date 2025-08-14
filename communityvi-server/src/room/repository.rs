use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model;
use crate::user;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait RoomRepository {
	async fn get(&self, connection: &mut dyn Connection, room_uuid: Uuid)
	-> Result<Option<model::Room>, DatabaseError>;
	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<model::Room, DatabaseError>;
	async fn update(&self, connection: &mut dyn Connection, room: &model::Room) -> Result<model::Room, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<(), DatabaseError>;

	async fn add_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uid: Uuid,
	) -> Result<(), DatabaseError>;
	async fn remove_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uid: Uuid,
	) -> Result<(), DatabaseError>;
	async fn list_users(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
	) -> Result<Vec<user::model::User>, DatabaseError>;
}
