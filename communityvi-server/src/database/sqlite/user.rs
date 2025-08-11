use super::{SqliteRepository, sqlite_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::user::model::User;
use crate::user::repository::UserRepository;
use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

#[async_trait]
impl UserRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<Option<User>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"SELECT uuid, name
			FROM user
			WHERE uuid = ?1",
		)
		.bind(user_uuid)
		.fetch_optional(connection)
		.await
		.map_err(Into::into)
	}

	async fn create(&self, connection: &mut dyn Connection, name: &str) -> Result<User, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let uuid = Uuid::new_v4();
		query_as(
			r"INSERT INTO user(uuid, name) VALUES (?1, ?2)
			RETURNING
				uuid,
				name",
		)
		.bind(uuid)
		.bind(name)
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

	#[tokio::test]
	async fn creates_user() {
		let mut connection = connection().await;

		let User { uuid, name } = SqliteRepository
			.user()
			.create(&mut *connection, "user")
			.await
			.expect("Failed to create user");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("user", name);
	}

	#[tokio::test]
	async fn gets_user() {
		let mut connection = connection().await;
		let user = SqliteRepository
			.create(&mut *connection, "user")
			.await
			.expect("Failed to create user");

		let User { uuid, name } = SqliteRepository
			.user()
			.get(&mut *connection, user.uuid)
			.await
			.expect("Failed to get user")
			.expect("User not found");

		assert_eq!(user.uuid, uuid);
		assert_eq!(user.name, name);
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
			.create(&mut *connection, "user")
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
