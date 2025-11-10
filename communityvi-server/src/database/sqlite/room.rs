use super::{SqliteRepository, sqlite_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use crate::user::model::User;
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::{FromRow, query, query_as};
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

	async fn get_all_users(
		&self,
		connection: &mut dyn Connection,
		room_uuid: Uuid,
	) -> Result<Vec<User>, DatabaseError> {
		#[derive(FromRow)]
		struct OptionalUser {
			uuid: Option<Uuid>,
			name: Option<String>,
			normalized_name: Option<String>,
		}

		let connection = sqlite_connection(connection)?;

		let result = query_as::<_, OptionalUser>(
			r"SELECT u.uuid, u.name, u.normalized_name
				FROM room r
				LEFT JOIN room_user ru ON ru.room_uuid = r.uuid
				LEFT JOIN user u ON ru.user_uuid = u.uuid
				WHERE r.uuid = ?1
				ORDER BY u.uuid ASC",
		)
		.bind(room_uuid)
		.fetch_all(&mut *connection)
		.await?;

		if result.is_empty() {
			return Err(DatabaseError::NotFound(anyhow!("Room not found")));
		}

		result
			.into_iter()
			.filter_map(
				|OptionalUser {
				     uuid,
				     name,
				     normalized_name,
				 }| {
					Some(Ok(User {
						uuid: uuid?,
						name: name?,
						normalized_name: normalized_name?,
					}))
				},
			)
			.collect()
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
		let (room_uuid, user) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add link");

		let expected = vec![user];
		let users_in_room = SqliteRepository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("failed to get_all_users");
		assert_eq!(expected, users_in_room);
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
			Ok(()) => panic!("Expected ForeignKeyViolation when room is missing"),
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
			Ok(()) => panic!("Expected ForeignKeyViolation when user is missing"),
			Err(err) => panic!("Expected ForeignKeyViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn does_not_add_duplicate_user_to_room_relationship() {
		let mut connection = connection().await;
		let (room_uuid, user) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add join");

		let result = SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(()) => panic!("Expected UniqueViolation for duplicate link"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn removes_user_from_room() {
		let mut connection = connection().await;
		let (room_uuid, user) = create_sample_room_and_user(&mut *connection).await;
		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add link");

		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("remove link");

		assert!(
			SqliteRepository
				.room()
				.get_all_users(&mut *connection, room_uuid)
				.await
				.expect("failed to get_all_users")
				.is_empty()
		);
	}

	#[tokio::test]
	async fn removes_user_from_room_idempotently() {
		let mut connection = connection().await;
		let (room_uuid, user) = create_sample_room_and_user(&mut *connection).await;

		// remove twice, second should still be Ok
		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("first remove ok");
		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("second remove ok");
	}

	#[tokio::test]
	async fn removes_user_from_room_when_they_have_not_joined_but_entities_exist() {
		let mut connection = connection().await;
		let (room_uuid, user) = create_sample_room_and_user(&mut *connection).await;

		SqliteRepository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
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

	async fn create_sample_room_and_user(connection: &mut dyn Connection) -> (Uuid, User) {
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
		(room_uuid, user)
	}

	#[tokio::test]
	async fn gets_all_users_is_empty_for_new_room() {
		let mut connection = connection().await;
		let Room { uuid: room_uuid, .. } = SqliteRepository
			.room()
			.create(&mut *connection, "lonely-room")
			.await
			.expect("create room");

		let users = SqliteRepository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("get_all_users");

		assert!(users.is_empty(), "expected no users, got: {users:?}");
	}

	#[tokio::test]
	async fn fails_get_all_users_for_nonexistent_room() {
		let mut connection = connection().await;
		let random_room = Uuid::new_v4();

		let result = SqliteRepository
			.room()
			.get_all_users(&mut *connection, random_room)
			.await;

		match result {
			Err(DatabaseError::NotFound(_)) => { /* ok */ }
			Ok(users) => panic!("Expected NotFound error, got Ok with users: {users:?}"),
			Err(err) => panic!("Expected NotFound, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn gets_all_users_ordered_by_user_uuid() {
		let mut connection = connection().await;
		let Room { uuid: room_uuid, .. } = SqliteRepository
			.room()
			.create(&mut *connection, "room-users")
			.await
			.expect("create room");

		let alice = SqliteRepository
			.user()
			.create(&mut *connection, "alice", &normalize_name("alice"))
			.await
			.expect("create alice");
		let bob = SqliteRepository
			.user()
			.create(&mut *connection, "bob", &normalize_name("bob"))
			.await
			.expect("create bob");

		// link both users
		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, bob.uuid)
			.await
			.expect("link bob");
		SqliteRepository
			.room()
			.add_user(&mut *connection, room_uuid, alice.uuid)
			.await
			.expect("link alice");

		let users = SqliteRepository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("get_all_users");

		let mut expected = [alice, bob];
		expected.sort_by_key(|user| user.uuid);

		assert_eq!(expected.as_slice(), &users);
	}
}
