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

	async fn add_user(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"INSERT INTO room_user(room_uuid, user_uuid) VALUES (?1, ?2)")
			.bind(room_uuid)
			.bind(user_uuid)
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
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Repository;
	use crate::database::sqlite::test_utils::connection;
	use crate::user::normalize_name;
	use sqlx::query_scalar;

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

	#[tokio::test]
	async fn adds_user_to_room() {
		let mut connection = connection().await;
		let (room_uuid, user_uuid) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("add link");

		assert_eq!(1, join_count(&mut *connection, room_uuid, user_uuid).await);
	}

	#[tokio::test]
	async fn does_not_add_user_to_room_when_room_missing() {
		let mut connection = connection().await;
		// create only user
		let user = SqliteRepository
			.user()
			.create(&mut *connection, "bob", &normalize_name("bob"))
			.await
			.expect("create user");
		let user_uuid = user.uuid;
		let missing_room = Uuid::new_v4();

		let result = SqliteRepository
			.room()
			.add_user(&mut *connection, missing_room, user_uuid)
			.await;

		match result {
			Err(DatabaseError::ForeignKeyViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected ForeignKeyViolation when room is missing"),
			Err(err) => panic!("Expected ForeignKeyViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn does_not_add_user_to_room_when_user_missing() {
		let mut connection = connection().await;
		// create only room
		let Room { uuid: room_uuid, .. } = SqliteRepository
			.room()
			.create(&mut *connection, "room-b")
			.await
			.expect("create room");
		let missing_user = Uuid::new_v4();

		let result = SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, missing_user)
			.await;

		match result {
			Err(DatabaseError::ForeignKeyViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected ForeignKeyViolation when user is missing"),
			Err(err) => panic!("Expected ForeignKeyViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn does_not_add_duplicate_user_to_room_relationship() {
		let mut connection = connection().await;
		let (room_uuid, user_uuid) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("add join");

		let result = SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user_uuid)
			.await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected UniqueViolation for duplicate link"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn removes_user_from_room() {
		let mut connection = connection().await;
		let (room_uuid, user_uuid) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("add link");
		assert_eq!(1, join_count(&mut *connection, room_uuid, user_uuid).await);

		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("remove link");

		assert_eq!(0, join_count(&mut *connection, room_uuid, user_uuid).await);
	}

	#[tokio::test]
	async fn removes_user_from_room_idempotently() {
		let mut connection = connection().await;
		let (room_uuid, user_uuid) = create_sample_room_and_user(&mut *connection).await;

		// remove twice, second should still be Ok
		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("first remove ok");
		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("second remove ok");
	}

	#[tokio::test]
	async fn removes_user_from_room_when_no_link_but_entities_exist() {
		let mut connection = connection().await;
		let (room_uuid, user_uuid) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user_uuid)
			.await
			.expect("remove ok without link");
	}

	#[tokio::test]
	async fn removes_user_from_room_regardless_of_presence() {
		let mut connection = connection().await;
		let random_room = Uuid::new_v4();
		let random_user = Uuid::new_v4();

		SqliteRepository
			.room()
			.remove_user(&mut *connection, random_room, random_user)
			.await
			.expect("removes_user_from_room_regardless_of_presence should succeed even if no rows match");
	}

	async fn create_sample_room_and_user(connection: &mut dyn Connection) -> (Uuid, Uuid) {
		let Room { uuid: room_uuid, .. } = SqliteRepository
			.room()
			.create(connection, "room-a")
			.await
			.expect("create room");
		let user = SqliteRepository
			.user()
			.create(connection, "alice", &normalize_name("alice"))
			.await
			.expect("create user");
		(room_uuid, user.uuid)
	}

	async fn join_count(connection: &mut dyn Connection, room_uuid: Uuid, user_uuid: Uuid) -> i64 {
		let connection = sqlite_connection(connection).expect("sqlite connection");
		query_scalar::<_, i64>(r"SELECT COUNT(*) FROM room_user WHERE room_uuid = ?1 AND user_uuid = ?2")
			.bind(room_uuid)
			.bind(user_uuid)
			.fetch_one(connection)
			.await
			.expect("count link")
	}
}
