use crate::store::Store;
use crate::store::error::{IntoStoreResult, StoreError};
use crate::store::models::Room;
use async_trait::async_trait;
use sqlx::{SqlitePool, migrate, query_as};
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteStore {
	pool: SqlitePool,
}

impl SqliteStore {
	pub async fn new(database_url: &str) -> Result<Self, StoreError> {
		let pool = SqlitePool::connect(database_url)
			.await
			.connection_error("Failed to connect to database")?;
		let store = Self { pool };
		store.migrate().await?;

		Ok(store)
	}

	async fn migrate(&self) -> Result<(), StoreError> {
		migrate!().run(&self.pool).await.map_err(Into::into)
	}
}

#[async_trait]
impl Store for SqliteStore {
	async fn get_room(&self, room_uuid: Uuid) -> Result<Option<Room>, StoreError> {
		query_as(r"SELECT * FROM room WHERE uuid = ?1")
			.bind(room_uuid)
			.fetch_optional(&self.pool)
			.await
			.map_err(Into::into)
	}

	async fn create_room(&self, name: &str) -> Result<Room, StoreError> {
		let uuid = Uuid::new_v4();
		query_as(
			r"INSERT INTO room (uuid, name) VALUES (?1, ?2)
			RETURNING
				uuid,
				name,
				medium_uuid",
		)
		.bind(uuid)
		.bind(name)
		.fetch_one(&self.pool)
		.await
		.map_err(Into::into)
	}

	async fn update_room(
		&self,
		Room {
			uuid,
			name,
			medium_uuid,
		}: Room,
	) -> Result<Room, StoreError> {
		query_as(
			r"UPDATE room
			SET
				name = ?1,
				medium_uuid = ?2
			WHERE uuid = ?3
			RETURNING
				uuid,
				name,
				medium_uuid",
		)
		.bind(name)
		.bind(medium_uuid)
		.bind(uuid)
		.fetch_one(&self.pool)
		.await
		.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn creates_room() {
		let store = store().await;

		let Room {
			uuid,
			name,
			medium_uuid,
		} = store.create_room("room").await.expect("Failed to create room");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("room", name);
		assert!(medium_uuid.is_none());
	}

	#[tokio::test]
	async fn updates_room_name() {
		let store = store().await;
		let room = store.create_room("room").await.expect("Failed to create room");

		let Room {
			uuid,
			name,
			medium_uuid,
		} = store
			.update_room(Room {
				name: "renamed".to_owned(),
				..room.clone()
			})
			.await
			.expect("Failed to update room");

		assert_eq!(room.uuid, uuid);
		assert_eq!("renamed", name);
		assert_eq!(room.medium_uuid, medium_uuid);
	}

	#[tokio::test]
	async fn gets_room() {
		let store = store().await;
		let created_room = store.create_room("room").await.expect("Failed to create room");

		let fetched_room = store
			.get_room(created_room.uuid)
			.await
			.expect("Failed to get room")
			.expect("Room not found");

		assert_eq!(created_room, fetched_room);
	}

	#[tokio::test]
	async fn get_room_returns_none_when_not_found() {
		let store = store().await;

		let fetched_room = store.get_room(Uuid::new_v4()).await.expect("Failed to get room");

		assert!(fetched_room.is_none());
	}

	async fn store() -> SqliteStore {
		SqliteStore::new("sqlite::memory:")
			.await
			.expect("Failed to create in-memory SQLite database")
	}
}
