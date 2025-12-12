use super::{SqliteRepository, sqlite_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use crate::user::repository::UserRepository;
use async_trait::async_trait;
use sqlx::{query, query_as};

#[async_trait]
impl UserRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<Option<User>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"SELECT uuid, name, normalized_name
			FROM user
			WHERE uuid = ?1",
		)
		.bind(user_uuid)
		.fetch_optional(connection)
		.await
		.map_err(Into::into)
	}

	async fn create(
		&self,
		connection: &mut dyn Connection,
		name: &str,
		normalized_name: &str,
	) -> Result<User, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			r"INSERT INTO user(uuid, name, normalized_name) VALUES (?1, ?2, ?3)
			RETURNING
				uuid,
				name,
				normalized_name",
		)
		.bind(uuid)
		.bind(name)
		.bind(normalized_name)
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}

	async fn remove(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"DELETE FROM user WHERE uuid = ?1")
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
	async fn creates_user() {
		let mut connection = connection().await;

		let name = "user";
		let User {
			uuid,
			name,
			normalized_name,
		} = SqliteRepository
			.user()
			.create(&mut *connection, name, &normalize_name(name))
			.await
			.expect("Failed to create user");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("user", name);
		assert_eq!(normalize_name("user"), normalized_name);
	}

	#[tokio::test]
	async fn doesnt_create_user_with_same_name() {
		let mut connection = connection().await;

		let name = "user";
		let normalized = normalize_name(name);
		SqliteRepository
			.user()
			.create(&mut *connection, name, &normalized)
			.await
			.expect("Failed to create first user");

		let result = SqliteRepository
			.user()
			.create(&mut *connection, name, &normalized)
			.await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected unique constraint violation when creating user with duplicate name"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn doesnt_create_user_with_same_normalized_name() {
		let mut connection = connection().await;

		let name1 = "ℝ𝓊𝓈𝓉";
		let name2 = "ℝ\t\t𝓊𝓈 𝓉"; // different spacing/glyphs, same normalization expected
		let normalized = normalize_name(name1);
		assert_eq!(
			normalized,
			normalize_name(name2),
			"Precondition: names should normalize to the same value"
		);

		SqliteRepository
			.user()
			.create(&mut *connection, name1, &normalized)
			.await
			.expect("Failed to create first user");

		let result = SqliteRepository
			.user()
			.create(&mut *connection, name2, &normalized)
			.await;

		match result {
			Err(DatabaseError::UniqueViolation(_)) => { /* ok */ }
			Ok(_) => panic!("Expected unique constraint violation when creating user with duplicate normalized_name"),
			Err(err) => panic!("Expected UniqueViolation, got: {err:?}"),
		}
	}

	#[tokio::test]
	async fn gets_user() {
		let mut connection = connection().await;
		let user = SqliteRepository
			.user()
			.create(&mut *connection, "user", &normalize_name("user"))
			.await
			.expect("Failed to create user");

		let User {
			uuid,
			name,
			normalized_name,
		} = SqliteRepository
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
	async fn get_user_returns_none_when_not_found() {
		let mut connection = connection().await;

		let fetched_user = SqliteRepository
			.user()
			.get(&mut *connection, Uuid::new_v4())
			.await
			.expect("Failed to get user");

		assert!(fetched_user.is_none());
	}

	#[tokio::test]
	async fn removes_user() {
		let mut connection = connection().await;
		let user = SqliteRepository
			.user()
			.create(&mut *connection, "user", &normalize_name("user"))
			.await
			.expect("Failed to create user");
		SqliteRepository
			.user()
			.remove(&mut *connection, user.uuid)
			.await
			.expect("Failed to remove user");

		let fetched_user = SqliteRepository
			.user()
			.get(&mut *connection, user.uuid)
			.await
			.expect("Failed to get user");

		assert!(fetched_user.is_none());
	}
}
