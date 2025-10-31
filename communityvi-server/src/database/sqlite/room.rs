use super::{SqliteRepository, sqlite_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

#[async_trait]
impl RoomRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<Option<Room>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"SELECT uuid, name, medium_uuid
			FROM room
			WHERE uuid = ?1",
		)
		.bind(room_uuid)
		.fetch_optional(connection)
		.await
		.map_err(Into::into)
	}

	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<Room, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			r"INSERT INTO room(uuid, name, medium_uuid) VALUES (?1, ?2, NULL)
			RETURNING
				uuid,
				name,
				medium_uuid",
		)
		.bind(uuid)
		.bind(name)
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}

	async fn remove(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"DELETE FROM room WHERE uuid = ?1")
			.bind(room_uuid)
			.execute(connection)
			.await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Repository;
	use crate::database::sqlite::test_utils::connection;

	#[tokio::test]
	async fn creates_room() {
		let mut connection = connection().await;

		let Room {
			uuid,
			name,
			medium_uuid,
		} = SqliteRepository
			.room()
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("test-room", name);
		assert_eq!(None, medium_uuid);
	}

	#[tokio::test]
	async fn gets_room() {
		let mut connection = connection().await;
		let room = SqliteRepository
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");

		let Room {
			uuid,
			name,
			medium_uuid,
		} = SqliteRepository
			.room()
			.get(&mut *connection, room.uuid)
			.await
			.expect("Failed to get room")
			.expect("Room not found");

		assert_eq!(room.uuid, uuid);
		assert_eq!(room.name, name);
		assert_eq!(room.medium_uuid, medium_uuid);
	}

	#[tokio::test]
	async fn get_room_returns_none_when_not_found() {
		let mut connection = connection().await;

		let fetched_room = SqliteRepository
			.room()
			.get(&mut *connection, Uuid::new_v4())
			.await
			.expect("Failed to get room");

		assert!(fetched_room.is_none());
	}

	#[tokio::test]
	async fn removes_room() {
		let mut connection = connection().await;
		let room = SqliteRepository
			.room()
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");
		SqliteRepository
			.room()
			.remove(&mut *connection, room.uuid)
			.await
			.expect("Failed to remove room");

		let fetched_room = SqliteRepository
			.room()
			.get(&mut *connection, room.uuid)
			.await
			.expect("Failed to get room");

		assert!(fetched_room.is_none());
	}
}
