use crate::store::error::StoreError;
use async_trait::async_trait;
use uuid::Uuid;

pub mod models;
pub mod sqlite;

pub mod error;

#[async_trait]
pub trait Store: Send + Sync {
	async fn get_room(&self, room_uuid: Uuid) -> Result<Option<models::Room>, StoreError>;
	async fn create_room(&self, name: &str) -> Result<models::Room, StoreError>;
	async fn update_room(&self, room: models::Room) -> Result<models::Room, StoreError>;
}
