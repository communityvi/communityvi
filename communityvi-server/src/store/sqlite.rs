use crate::store::models::Room;
use sqlx::{SqliteExecutor, SqlitePool};
use uuid::Uuid;

pub struct SqliteStore {
	database_pool: SqlitePool,
}

impl SqliteStore {
	pub fn new(database_pool: SqlitePool) -> Self {
		Self { database_pool }
	}

	pub async fn get_room(database_connection: impl SqliteExecutor<'_>, room_uuid: Uuid) -> sqlx::Result<Option<Room>> {
		sqlx::query_as!(Room, r#"SELECT * FROM room WHERE uuid = ?1"#, room_uuid)
			.fetch_optional(database_connection)
			.await
	}

	pub async fn create_room(database_connection: impl SqliteExecutor<'_>, name: String) -> sqlx::Result<Room> {
		let uuid = Uuid::new_v4();
		sqlx::query_as!(
			Room,
			r#"INSERT INTO room (uuid, name) VALUES (?1, ?2)
			RETURNING 
				uuid as "uuid: Uuid",
				name,
				medium_uuid as "uuid?""#,
			uuid,
			name,
		)
		.fetch_one(database_connection)
		.await
	}

	pub async fn update_room(room: Room) -> sqlx::Result<Room> {
		todo!()
	}
}
