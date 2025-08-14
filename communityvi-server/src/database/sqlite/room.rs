use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::sqlite::{SqliteRepository, sqlite_connection};
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use crate::user;
use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

#[async_trait]
impl RoomRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, room_uuid: Uuid) -> Result<Option<Room>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(r"SELECT uuid, name, medium_uuid FROM room WHERE uuid = ?1")
			.bind(room_uuid)
			.fetch_optional(connection)
			.await
			.map_err(Into::into)
	}

	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<Room, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			r"
			INSERT INTO room (
				uuid,
				name
			) VALUES (?1, ?2)
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

	async fn update(
		&self,
		connection: &mut dyn Connection,
		Room {
			uuid,
			name,
			medium_uuid,
		}: &Room,
	) -> Result<Room, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"
			UPDATE room
			SET
				name = ?1,
				medium_uuid = ?2
			WHERE
				uuid = ?3
			RETURNING
				uuid,
				name,
				medium_uuid
			",
		)
		.bind(name)
		.bind(medium_uuid)
		.bind(uuid)
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

	async fn add_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uid: Uuid,
	) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"INSERT INTO room_user (room_uuid, user_uuid) VALUES (?1, ?2)")
			.bind(room_uuid)
			.bind(user_uid)
			.execute(connection)
			.await?;

		Ok(())
	}

	async fn remove_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"DELETE FROM room_user WHERE room_uuid = ?1 AND user_uuid = ?2")
			.bind(room_uuid)
			.bind(user_uuid)
			.execute(connection)
			.await?;

		Ok(())
	}

	async fn list_users(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
	) -> Result<Vec<user::model::User>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"
			SELECT
				user.uuid,
				user.name
			FROM
				room_user
			INNER JOIN user ON room_user.user_uuid = user.uuid
			WHERE
				room_uuid = ?1
			",
		)
		.bind(room_uuid)
		.fetch_all(connection)
		.await
		.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Repository;
	use crate::database::sqlite::test_utils::connection;
	use crate::room::medium::playback_state::PlaybackState;
	use chrono::Duration;

	#[tokio::test]
	async fn creates_room() {
		let mut connection = connection().await;

		let Room {
			uuid,
			name,
			medium_uuid,
		} = SqliteRepository
			.room()
			.create(&mut *connection, "test")
			.await
			.expect("Failed to create room");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("test", name);
		assert_eq!(None, medium_uuid);
	}

	#[tokio::test]
	async fn updates_room() {
		let mut connection = connection().await;

		let room = SqliteRepository
			.room()
			.create(&mut *connection, "room")
			.await
			.expect("Failed to create room");
		let medium = SqliteRepository
			.medium()
			.create(
				&mut *connection,
				"movie",
				1337,
				PlaybackState::Paused {
					at_position: Duration::milliseconds(0),
				},
			)
			.await
			.expect("Failed to create medium");

		let Room {
			uuid,
			name,
			medium_uuid,
		} = SqliteRepository
			.room()
			.update(
				&mut *connection,
				&Room {
					uuid: room.uuid,
					name: "new name".to_string(),
					medium_uuid: Some(medium.uuid),
				},
			)
			.await
			.expect("Failed to update room");

		assert_eq!(room.uuid, uuid);
		assert_eq!("new name", name);
		assert_eq!(Some(medium.uuid), medium_uuid);
	}

	#[tokio::test]
	async fn gets_room() {
		let mut connection = connection().await;

		let room = SqliteRepository
			.room()
			.create(&mut *connection, "test")
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
	async fn get_returns_none_when_room_not_found() {
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
			.create(&mut *connection, "test")
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

	#[tokio::test]
	async fn adds_user_to_room() {
		let mut connection = connection().await;

		let room = SqliteRepository
			.room()
			.create(&mut *connection, "room")
			.await
			.expect("Failed to create room");
		let user = SqliteRepository
			.user()
			.create(&mut *connection, "user")
			.await
			.expect("Failed to create user");

		SqliteRepository
			.room()
			.add_user(&mut *connection, room.uuid, user.uuid)
			.await
			.expect("Failed to add user to room");

		let users = SqliteRepository
			.room()
			.list_users(&mut *connection, room.uuid)
			.await
			.expect("Failed to list users");
		assert_eq!(vec![user], users);
	}

	#[tokio::test]
	async fn removes_user_from_room() {
		let mut connection = connection().await;

		let room = SqliteRepository
			.room()
			.create(&mut *connection, "room")
			.await
			.expect("Failed to create room");
		let user = SqliteRepository
			.user()
			.create(&mut *connection, "user")
			.await
			.expect("Failed to create user");
		SqliteRepository
			.room()
			.add_user(&mut *connection, room.uuid, user.uuid)
			.await
			.expect("Failed to add user to room");

		SqliteRepository
			.room()
			.remove_user(&mut *connection, room.uuid, user.uuid)
			.await
			.expect("Failed to remove user from room");

		let users = SqliteRepository
			.room()
			.list_users(&mut *connection, room.uuid)
			.await
			.expect("Failed to list users");
		assert!(users.is_empty());
	}

	#[tokio::test]
	async fn lists_users_in_room() {
		let mut connection = connection().await;

		let room = SqliteRepository
			.room()
			.create(&mut *connection, "room")
			.await
			.expect("Failed to create room");
		let user1 = SqliteRepository
			.user()
			.create(&mut *connection, "user1")
			.await
			.expect("Failed to create user");
		let user2 = SqliteRepository
			.user()
			.create(&mut *connection, "user2")
			.await
			.expect("Failed to create user");
		SqliteRepository
			.room()
			.add_user(&mut *connection, room.uuid, user1.uuid)
			.await
			.expect("Failed to add user1 to room");
		SqliteRepository
			.room()
			.add_user(&mut *connection, room.uuid, user2.uuid)
			.await
			.expect("Failed to add user2 to room");

		let users = SqliteRepository
			.room()
			.list_users(&mut *connection, room.uuid)
			.await
			.expect("Failed to list users");

		assert!(users.contains(&user1));
		assert!(users.contains(&user2));
	}
}
