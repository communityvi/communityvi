use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::libsql::LibSqlRepository;
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use async_trait::async_trait;

#[async_trait]
impl RoomRepository for LibSqlRepository {
	async fn get(&self, _connection: &mut dyn Connection, _room_uuid: Uuid) -> Result<Option<Room>, DatabaseError> {
		unimplemented!()
	}

	async fn create(&self, _connection: &mut dyn Connection, _name: &str) -> Result<Room, DatabaseError> {
		unimplemented!()
	}

	async fn remove(&self, _connection: &mut dyn Connection, _room_uuid: Uuid) -> Result<(), DatabaseError> {
		unimplemented!()
	}

	async fn get_all_users(
		&self,
		_connection: &mut dyn Connection,
		_room_uuid: Uuid,
	) -> Result<Vec<User>, DatabaseError> {
		unimplemented!()
	}

	async fn add_user(
		&self,
		_connection: &mut dyn Connection,
		_room_uuid: Uuid,
		_user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		unimplemented!()
	}

	async fn remove_user(
		&self,
		_connection: &mut dyn Connection,
		_room_uuid: Uuid,
		_user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		unimplemented!()
	}
}
