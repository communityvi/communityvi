use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::sqlite::{SqliteRepository, sqlite_connection};
use crate::room::medium::model::Medium;
use crate::room::medium::playback_state::PlaybackState;
use crate::room::medium::repository::MediumRepository;
use async_trait::async_trait;
use sqlx::{query, query_as};
use uuid::Uuid;

#[async_trait]
impl MediumRepository for SqliteRepository {
	async fn get(&self, connection: &mut dyn Connection, medium_uuid: Uuid) -> Result<Option<Medium>, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"
			SELECT
				uuid,
				name,
				version,
				length_ms,
				playback_state,
				playback_state_start_time_ms,
				playback_state_at_position_ms
			FROM medium
			WHERE uuid = ?1
			",
		)
		.bind(medium_uuid)
		.fetch_optional(connection)
		.await
		.map_err(Into::into)
	}

	async fn create(
		&self,
		connection: &mut dyn Connection,
		name: &str,
		length_ms: i64,
		playback_state: PlaybackState,
	) -> Result<Medium, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		let (playback_state, start_time, position) = match playback_state {
			PlaybackState::Playing { start_time } => ("playing", Some(start_time.num_milliseconds()), None),
			PlaybackState::Paused { at_position } => ("paused", None, Some(at_position.num_milliseconds())),
		};

		let uuid = Uuid::new_v4();
		query_as(
			r"
			INSERT INTO medium (
				uuid,
				name,
				version,
				length_ms,
				playback_state,
				playback_state_start_time_ms,
				playback_state_at_position_ms
			)
			VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6)
			RETURNING
				uuid,
				name,
				version,
				length_ms,
				playback_state,
				playback_state_start_time_ms,
				playback_state_at_position_ms
			",
		)
		.bind(uuid)
		.bind(name)
		.bind(length_ms)
		.bind(playback_state)
		.bind(start_time)
		.bind(position)
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}

	async fn update(
		&self,
		connection: &mut dyn Connection,
		Medium {
			uuid,
			name,
			version: _,
			length_ms,
			playback_state,
			playback_state_start_time_ms,
			playback_state_at_position_ms,
		}: &Medium,
	) -> Result<Medium, DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query_as(
			r"
			UPDATE medium
			SET
				name = ?1,
				version = version + 1,
				length_ms = ?2,
				playback_state = ?3,
				playback_state_start_time_ms = ?4,
				playback_state_at_position_ms = ?5
			WHERE uuid = ?6
			RETURNING
				uuid,
				name,
				version,
				length_ms,
				playback_state,
				playback_state_start_time_ms,
				playback_state_at_position_ms
			",
		)
		.bind(name)
		.bind(length_ms)
		.bind(playback_state)
		.bind(playback_state_start_time_ms)
		.bind(playback_state_at_position_ms)
		.bind(uuid)
		.fetch_one(connection)
		.await
		.map_err(Into::into)
	}

	async fn remove(&self, connection: &mut dyn Connection, medium_uuid: Uuid) -> Result<(), DatabaseError> {
		let connection = sqlite_connection(connection)?;

		query(r"DELETE FROM medium WHERE uuid = ?1")
			.bind(medium_uuid)
			.execute(connection)
			.await?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::database::Repository;
	use crate::database::sqlite::test_utils::connection;
	use crate::room::medium::model;
	use chrono::Duration;

	#[tokio::test]
	async fn creates_medium() {
		let mut connection = connection().await;

		let Medium {
			uuid,
			name,
			version,
			length_ms,
			playback_state,
			playback_state_start_time_ms,
			playback_state_at_position_ms,
		} = SqliteRepository
			.medium()
			.create(
				&mut *connection,
				"medium",
				1337,
				PlaybackState::Paused {
					at_position: Duration::milliseconds(42),
				},
			)
			.await
			.expect("Failed to create medium");

		assert_eq!(4, uuid.get_version_num());
		assert_eq!("medium", name);
		assert_eq!(0, version);
		assert_eq!(1337, length_ms);
		assert_eq!(model::PlaybackState::Paused, playback_state);
		assert_eq!(Some(42), playback_state_at_position_ms);
		assert_eq!(None, playback_state_start_time_ms);
	}

	#[tokio::test]
	async fn gets_medium() {
		let mut connection = connection().await;

		let medium = SqliteRepository
			.medium()
			.create(
				&mut *connection,
				"medium",
				1337,
				PlaybackState::Paused {
					at_position: Duration::milliseconds(42),
				},
			)
			.await
			.expect("Failed to create medium");

		let Medium {
			uuid,
			name,
			version,
			length_ms,
			playback_state,
			playback_state_start_time_ms,
			playback_state_at_position_ms,
		} = SqliteRepository
			.medium()
			.get(&mut *connection, medium.uuid)
			.await
			.expect("Failed to get medium")
			.expect("Medium not found");

		assert_eq!(medium.uuid, uuid);
		assert_eq!(medium.name, name);
		assert_eq!(medium.version, version);
		assert_eq!(medium.length_ms, length_ms);
		assert_eq!(medium.playback_state, playback_state);
		assert_eq!(medium.playback_state_start_time_ms, playback_state_start_time_ms);
		assert_eq!(medium.playback_state_at_position_ms, playback_state_at_position_ms);
	}

	#[tokio::test]
	async fn get_returns_none_when_medium_not_found() {
		let mut connection = connection().await;

		let fetched_medium = SqliteRepository
			.medium()
			.get(&mut *connection, Uuid::new_v4())
			.await
			.expect("Failed to get medium");

		assert!(fetched_medium.is_none());
	}

	#[tokio::test]
	async fn removes_medium() {
		let mut connection = connection().await;

		let medium = SqliteRepository
			.medium()
			.create(
				&mut *connection,
				"medium",
				1337,
				PlaybackState::Paused {
					at_position: Duration::milliseconds(42),
				},
			)
			.await
			.expect("Failed to create medium");

		SqliteRepository
			.medium()
			.remove(&mut *connection, medium.uuid)
			.await
			.expect("Failed to remove medium");

		let fetched_medium = SqliteRepository
			.medium()
			.get(&mut *connection, medium.uuid)
			.await
			.expect("Failed to get medium");
		assert!(fetched_medium.is_none());
	}

	#[tokio::test]
	async fn updates_medium() {
		let mut connection = connection().await;

		let medium = SqliteRepository
			.medium()
			.create(
				&mut *connection,
				"medium",
				1337,
				PlaybackState::Paused {
					at_position: Duration::milliseconds(42),
				},
			)
			.await
			.expect("Failed to create medium");

		let Medium {
			uuid,
			name,
			version,
			length_ms,
			playback_state,
			playback_state_start_time_ms,
			playback_state_at_position_ms,
		} = SqliteRepository
			.medium()
			.update(
				&mut *connection,
				&Medium {
					uuid: medium.uuid,
					name: "new medium".to_string(),
					version: 999, // NOTE: This needs to be ignored
					length_ms: 8000,
					playback_state: model::PlaybackState::Playing,
					playback_state_start_time_ms: Some(42),
					playback_state_at_position_ms: None,
				},
			)
			.await
			.expect("Failed to update medium");

		assert_eq!(medium.uuid, uuid);
		assert_eq!("new medium", name);
		assert_eq!(1, version, "Version should have been incremented by the store");
		assert_eq!(8000, length_ms);
		assert_eq!(model::PlaybackState::Playing, playback_state);
		assert_eq!(Some(42), playback_state_start_time_ms);
		assert_eq!(None, playback_state_at_position_ms);
	}
}
