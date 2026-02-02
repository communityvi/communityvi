use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::libsql::{LibSqlRepository, libsql_connection};
use crate::room::model::Room;
use crate::room::repository::RoomRepository;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use futures_util::{StreamExt, TryStreamExt, stream};
use libsql::{Row, Value};

#[async_trait]
impl RoomRepository for LibSqlRepository {
	async fn get(&self, connection: &dyn Connection, room_uuid: Uuid) -> Result<Option<Room>, DatabaseError> {
		let connection = libsql_connection(connection)?;

		let mut rows = connection
			.query(
				r"SELECT uuid, name, medium_uuid
			FROM room
			WHERE uuid = ?1",
				[room_uuid],
			)
			.await?;

		let Some(row) = rows.next().await? else {
			return Ok(None);
		};

		Ok(Some(row.try_into().map_err(DatabaseError::Decode)?))
	}

	async fn create(&self, connection: &dyn Connection, name: &str) -> Result<Room, DatabaseError> {
		let connection = libsql_connection(connection)?;

		let uuid = Uuid::new_v4();
		let mut rows = connection
			.query(
				r"INSERT INTO room(uuid, name, medium_uuid) VALUES (?1, ?2, NULL)
			RETURNING
				uuid,
				name,
				medium_uuid",
				(uuid, name),
			)
			.await?;

		rows.next()
			.await?
			.ok_or_else(|| DatabaseError::NotFound(anyhow!("not found")))?
			.try_into()
			.map_err(DatabaseError::Decode)
	}

	async fn remove(&self, connection: &dyn Connection, room_uuid: Uuid) -> Result<(), DatabaseError> {
		let connection = libsql_connection(connection)?;

		connection
			.execute(r"DELETE FROM room WHERE uuid = ?1", [room_uuid])
			.await?;

		Ok(())
	}

	async fn get_all_users(&self, connection: &dyn Connection, room_uuid: Uuid) -> Result<Vec<User>, DatabaseError> {
		struct OptionalUser {
			uuid: Option<Uuid>,
			name: Option<String>,
			normalized_name: Option<String>,
		}

		impl TryFrom<Row> for OptionalUser {
			type Error = anyhow::Error;

			fn try_from(row: Row) -> Result<Self, Self::Error> {
				let uuid = row.get_value(0)?;
				let name = row.get_value(1)?;
				let normalized_name = row.get_value(2)?;

				Ok(Self {
					uuid: if uuid.is_null() { None } else { Some(uuid.try_into()?) },
					name: match name {
						Value::Null => None,
						Value::Text(name) => Some(name),
						_ => bail!("Invalid type for uuid"),
					},
					normalized_name: match normalized_name {
						Value::Null => None,
						Value::Text(normalized_name) => Some(normalized_name),
						_ => bail!("Invalid type for normalized_name"),
					},
				})
			}
		}

		let connection = libsql_connection(connection)?;

		let mut rows = connection
			.query(
				r"SELECT u.uuid, u.name, u.normalized_name
				FROM room r
				LEFT JOIN room_user ru ON ru.room_uuid = r.uuid
				LEFT JOIN user u ON ru.user_uuid = u.uuid
				WHERE r.uuid = ?1
				ORDER BY u.uuid ASC",
				[room_uuid],
			)
			.await?;

		let Some(first_row) = rows.next().await? else {
			return Err(DatabaseError::NotFound(anyhow!("Room not found")));
		};

		stream::once(async move { Ok(first_row) })
			.chain(stream::try_unfold(rows, async |mut rows| {
				let Some(row) = rows.next().await? else {
					return Ok(None);
				};
				Ok(Some((row, rows)))
			}))
			.and_then(async |row| row.try_into().map_err(DatabaseError::Decode))
			.try_filter_map(async |optional_user| {
				let OptionalUser {
					uuid: Some(uuid),
					name: Some(name),
					normalized_name: Some(normalized_name),
				} = optional_user
				else {
					return Ok(None);
				};

				Ok(Some(User {
					uuid,
					name,
					normalized_name,
				}))
			})
			.try_collect()
			.await
	}

	async fn add_user(
		&self,
		connection: &dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		let connection = libsql_connection(connection)?;

		connection
			.execute(
				r"INSERT INTO room_user(room_uuid, user_uuid) VALUES (?1, ?2)",
				(room_uuid, user_uuid),
			)
			.await?;
		Ok(())
	}

	async fn remove_user(
		&self,
		connection: &dyn Connection,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> Result<(), DatabaseError> {
		let connection = libsql_connection(connection)?;

		connection
			.execute(
				r"DELETE FROM room_user WHERE room_uuid = ?1 AND user_uuid = ?2",
				(room_uuid, user_uuid),
			)
			.await?;
		Ok(())
	}
}
