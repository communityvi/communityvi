#[generic_tests::define(attrs(tokio::test))]
mod room_tests {
	use crate::database::Repository;
	use crate::database::error::DatabaseError;
	use crate::database::libsql::test_utils::LibSqlTestFactory;
	use crate::database::{Connection, TestFactory};
	use crate::room::model::Room;
	use crate::types::uuid::Uuid;
	use crate::user::model::User;
	use crate::user::normalize_name;

	#[tokio::test]
	async fn creates_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let Room {
			uuid,
			name,
			medium_uuid,
		} = repository
			.room()
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("test-room", name);
		assert_eq!(None, medium_uuid);
	}

	#[tokio::test]
	async fn gets_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let room = repository
			.room()
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");

		let Room {
			uuid,
			name,
			medium_uuid,
		} = repository
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
	async fn get_room_returns_none_when_not_found<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let fetched_room = repository
			.room()
			.get(&mut *connection, Uuid::new_v4())
			.await
			.expect("Failed to get room");

		assert!(fetched_room.is_none());
	}

	#[tokio::test]
	async fn removes_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let room = repository
			.room()
			.create(&mut *connection, "test-room")
			.await
			.expect("Failed to create room");
		repository
			.room()
			.remove(&mut *connection, room.uuid)
			.await
			.expect("Failed to remove room");

		let fetched_room = repository
			.room()
			.get(&mut *connection, room.uuid)
			.await
			.expect("Failed to get room");

		assert!(fetched_room.is_none());
	}

	#[tokio::test]
	async fn adds_user_to_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let (room_uuid, user) = create_sample_room_and_user(repository.as_ref(), &mut *connection).await;

		repository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add link");

		let expected = vec![user];
		let users_in_room = repository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("failed to get_all_users");
		assert_eq!(expected, users_in_room);
	}

	#[tokio::test]
	async fn does_not_add_user_to_room_when_room_missing<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		// create only user
		let user = repository
			.user()
			.create(&mut *connection, "bob", &normalize_name("bob"))
			.await
			.expect("create user");
		let user_uuid = user.uuid;
		let missing_room = Uuid::new_v4();

		let result = repository
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
	async fn does_not_add_user_to_room_when_user_missing<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		// create only room
		let Room { uuid: room_uuid, .. } = repository
			.room()
			.create(&mut *connection, "room-b")
			.await
			.expect("create room");
		let missing_user = Uuid::new_v4();

		let result = repository
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
	async fn does_not_add_duplicate_user_to_room_relationship<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let (room_uuid, user) = create_sample_room_and_user(repository.as_ref(), &mut *connection).await;

		repository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add join");

		let result = repository.room().add_user(&mut *connection, room_uuid, user.uuid).await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(()) => panic!("Expected UniqueViolation for duplicate link"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn removes_user_from_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let (room_uuid, user) = create_sample_room_and_user(repository.as_ref(), &mut *connection).await;
		repository
			.room()
			.add_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("add link");

		repository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("remove link");

		assert!(
			repository
				.room()
				.get_all_users(&mut *connection, room_uuid)
				.await
				.expect("failed to get_all_users")
				.is_empty()
		);
	}

	#[tokio::test]
	async fn removes_user_from_room_idempotently<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let (room_uuid, user) = create_sample_room_and_user(repository.as_ref(), &mut *connection).await;

		// remove twice, second should still be Ok
		repository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("first remove ok");
		repository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("second remove ok");
	}

	#[tokio::test]
	async fn removes_user_from_room_when_they_have_not_joined_but_entities_exist<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let (room_uuid, user) = create_sample_room_and_user(repository.as_ref(), &mut *connection).await;

		repository
			.room()
			.remove_user(&mut *connection, room_uuid, user.uuid)
			.await
			.expect("remove ok without link");
	}

	#[tokio::test]
	async fn removes_user_from_room_regardless_of_presence<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let random_room = Uuid::new_v4();
		let random_user = Uuid::new_v4();

		repository
			.room()
			.remove_user(&mut *connection, random_room, random_user)
			.await
			.expect("removes_user_from_room_regardless_of_presence should succeed even if no rows match");
	}

	async fn create_sample_room_and_user(repository: &dyn Repository, connection: &mut dyn Connection) -> (Uuid, User) {
		let Room { uuid: room_uuid, .. } = repository
			.room()
			.create(connection, "room-a")
			.await
			.expect("create room");
		let user = repository
			.user()
			.create(connection, "alice", &normalize_name("alice"))
			.await
			.expect("create user");
		(room_uuid, user)
	}

	#[tokio::test]
	async fn gets_all_users_is_empty_for_new_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let Room { uuid: room_uuid, .. } = repository
			.room()
			.create(&mut *connection, "lonely-room")
			.await
			.expect("create room");

		let users = repository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("get_all_users");

		assert!(users.is_empty(), "expected no users, got: {users:?}");
	}

	#[tokio::test]
	async fn fails_get_all_users_for_nonexistent_room<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let random_room = Uuid::new_v4();

		let result = repository.room().get_all_users(&mut *connection, random_room).await;

		match result {
			Err(DatabaseError::NotFound(_)) => { /* ok */ }
			Ok(users) => panic!("Expected NotFound error, got Ok with users: {users:?}"),
			Err(err) => panic!("Expected NotFound, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn gets_all_users_ordered_by_user_uuid<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let Room { uuid: room_uuid, .. } = repository
			.room()
			.create(&mut *connection, "room-users")
			.await
			.expect("create room");

		let alice = repository
			.user()
			.create(&mut *connection, "alice", &normalize_name("alice"))
			.await
			.expect("create alice");
		let bob = repository
			.user()
			.create(&mut *connection, "bob", &normalize_name("bob"))
			.await
			.expect("create bob");

		// link both users
		repository
			.room()
			.add_user(&mut *connection, room_uuid, bob.uuid)
			.await
			.expect("link bob");
		repository
			.room()
			.add_user(&mut *connection, room_uuid, alice.uuid)
			.await
			.expect("link alice");

		let users = repository
			.room()
			.get_all_users(&mut *connection, room_uuid)
			.await
			.expect("get_all_users");

		let mut expected = [alice, bob];
		expected.sort_by_key(|user| user.uuid);

		assert_eq!(expected.as_slice(), &users);
	}

	#[instantiate_tests(<LibSqlTestFactory>)]
	mod libsql {}
}
