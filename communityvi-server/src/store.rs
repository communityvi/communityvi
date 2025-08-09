use crate::store::error::StoreError;
use dynosaur::dynosaur;
use uuid::Uuid;

pub mod models;
pub mod sqlite;

pub mod error;

// TODO: Rename to `Database` and make relational database specific (at least for now)
// TODO: Add transaction/regular connection support?
#[dynosaur(pub DynStore = dyn(box) Store)]
pub trait Store: Send + Sync {
	fn get_room(&self, room_uuid: Uuid) -> impl Future<Output = Result<Option<models::Room>, StoreError>> + Send;
	fn create_room(&self, name: &str) -> impl Future<Output = Result<models::Room, StoreError>> + Send;
	fn update_room(&self, room: models::Room) -> impl Future<Output = Result<models::Room, StoreError>> + Send;

	fn get_user(&self, user_uuid: Uuid) -> impl Future<Output = Result<Option<models::User>, StoreError>> + Send;
	fn create_user(&self, name: &str) -> impl Future<Output = Result<models::User, StoreError>> + Send;
	fn remove_user(&self, user_uuid: Uuid) -> impl Future<Output = Result<(), StoreError>> + Send;

	fn add_user_to_room(&self, room_uuid: Uuid, user_uuid: Uuid)
	-> impl Future<Output = Result<(), StoreError>> + Send;
	fn remove_user_from_room(
		&self,
		room_uuid: Uuid,
		user_uuid: Uuid,
	) -> impl Future<Output = Result<(), StoreError>> + Send;
	fn list_room_users(&self, room_uuid: Uuid, user_uuid: Uuid) -> impl Future<Output = Result<(), StoreError>> + Send;
}
