#[generic_tests::define(attrs(tokio::test))]
mod user_tests {
	use crate::database::TestFactory;
	use crate::database::error::DatabaseError;
	use crate::database::sqlite::test_utils::SqliteTestFactory;
	use crate::types::uuid::Uuid;
	use crate::user::model::User;
	use crate::user::normalize_name;

	#[tokio::test]
	async fn creates_user<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let name = "user";
		let User {
			uuid,
			name,
			normalized_name,
		} = repository
			.user()
			.create(&mut *connection, name, &normalize_name(name))
			.await
			.expect("Failed to create user");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("user", name);
		assert_eq!(normalize_name("user"), normalized_name);
	}

	#[tokio::test]
	async fn doesnt_create_user_with_same_name<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let name = "user";
		let normalized = normalize_name(name);
		repository
			.user()
			.create(&mut *connection, name, &normalized)
			.await
			.expect("Failed to create first user");

		let result = repository.user().create(&mut *connection, name, &normalized).await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected unique constraint violation when creating user with duplicate name"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn doesnt_create_user_with_same_normalized_name<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let name1 = "ℝ𝓊𝓈𝓉";
		let name2 = "ℝ\t\t𝓊𝓈 𝓉"; // different spacing/glyphs, same normalization expected
		let normalized = normalize_name(name1);
		assert_eq!(
			normalized,
			normalize_name(name2),
			"Precondition: names should normalize to the same value"
		);

		repository
			.user()
			.create(&mut *connection, name1, &normalized)
			.await
			.expect("Failed to create first user");

		let result = repository.user().create(&mut *connection, name2, &normalized).await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected unique constraint violation when creating user with duplicate normalized_name"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn gets_user<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let user = repository
			.user()
			.create(&mut *connection, "user", &normalize_name("user"))
			.await
			.expect("Failed to create user");

		let User {
			uuid,
			name,
			normalized_name,
		} = repository
			.user()
			.get(&mut *connection, user.uuid)
			.await
			.expect("Failed to get user")
			.expect("User not found");

		assert_eq!(user.uuid, uuid);
		assert_eq!(user.name, name);
		assert_eq!(user.normalized_name, normalized_name);
	}

	#[tokio::test]
	async fn get_user_returns_none_when_not_found<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let fetched_user = repository
			.user()
			.get(&mut *connection, Uuid::new_v4())
			.await
			.expect("Failed to get user");

		assert!(fetched_user.is_none());
	}

	#[tokio::test]
	async fn removes_user<Factory: TestFactory>() {
		let mut connection = Factory::connection().await;
		let repository = Factory::repository();

		let user = repository
			.user()
			.create(&mut *connection, "user", &normalize_name("user"))
			.await
			.expect("Failed to create user");
		repository
			.user()
			.remove(&mut *connection, user.uuid)
			.await
			.expect("Failed to remove user");

		let fetched_user = repository
			.user()
			.get(&mut *connection, user.uuid)
			.await
			.expect("Failed to get user");

		assert!(fetched_user.is_none());
	}

	#[instantiate_tests(<SqliteTestFactory>)]
	mod sqlite {}
}
