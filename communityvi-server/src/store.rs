use crate::store::error::StoreError;
use dynosaur::dynosaur;
use uuid::Uuid;

pub mod models;
pub mod sqlite;

pub mod error;

#[dynosaur(pub DynStore = dyn(box) Store)]
pub trait Store: Send + Sync {
	fn get_room(&self, room_uuid: Uuid) -> impl Future<Output = Result<Option<models::Room>, StoreError>> + Send;
	fn create_room(&self, name: &str) -> impl Future<Output = Result<models::Room, StoreError>> + Send;
	fn update_room(&self, room: models::Room) -> impl Future<Output = Result<models::Room, StoreError>> + Send;
}
