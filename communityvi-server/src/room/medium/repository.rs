use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::room::medium::model;
use crate::room::medium::playback_state::PlaybackState;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait MediumRepository {
	async fn get(
		&self,
		connection: &mut dyn Connection,
		medium_uuid: Uuid,
	) -> Result<Option<model::Medium>, DatabaseError>;
	async fn create(
		&self,
		connection: &mut dyn Connection,
		name: &str,
		length_ms: i64,
		playback_state: PlaybackState,
	) -> Result<model::Medium, DatabaseError>;
	async fn update(
		&self,
		connection: &mut dyn Connection,
		medium: &model::Medium,
	) -> Result<model::Medium, DatabaseError>;
	async fn remove(&self, connection: &mut dyn Connection, medium_uuid: Uuid) -> Result<(), DatabaseError>;
}
