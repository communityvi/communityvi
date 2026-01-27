use super::{LibSqlRepository, libsql_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use crate::user::repository::UserRepository;
use anyhow::anyhow;
use async_trait::async_trait;

#[async_trait]
impl UserRepository for LibSqlRepository {
	async fn get(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<Option<User>, DatabaseError> {
		let connection = libsql_connection(connection)?;

		let mut rows = connection
			.query(
				r"SELECT uuid, name, normalized_name
			FROM user
			WHERE uuid = ?1",
				[user_uuid],
			)
			.await?;

		let Some(row) = rows.next().await? else {
			return Ok(None);
		};

		Ok(Some(row.try_into().map_err(DatabaseError::Decode)?))
	}

	async fn create(
		&self,
		connection: &mut dyn Connection,
		name: &str,
		normalized_name: &str,
	) -> Result<User, DatabaseError> {
		let connection = libsql_connection(connection)?;

		let uuid = Uuid::new_v4();
		let mut rows = connection
			.query(
				r"INSERT INTO user(uuid, name, normalized_name) VALUES (?1, ?2, ?3)
			RETURNING
				uuid,
				name,
				normalized_name",
				(uuid, name, normalized_name),
			)
			.await?;

		rows.next()
			.await?
			.ok_or_else(|| DatabaseError::NotFound(anyhow!("not found")))?
			.try_into()
			.map_err(DatabaseError::Decode)
	}

	async fn remove(&self, connection: &mut dyn Connection, user_uuid: Uuid) -> Result<(), DatabaseError> {
		let connection = libsql_connection(connection)?;

		connection
			.execute(r"DELETE FROM user WHERE uuid = ?1", [user_uuid])
			.await?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {}
