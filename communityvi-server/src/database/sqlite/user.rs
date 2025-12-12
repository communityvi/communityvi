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
mod tests {}
