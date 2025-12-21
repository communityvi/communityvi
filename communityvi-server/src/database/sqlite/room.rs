use super::{SqliteRepository, sqlite_connection};
use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::{FromRow, query, query_as};

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
