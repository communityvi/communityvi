use anyhow::Context;
use sqlx::error::ErrorKind;
use sqlx::migrate::MigrateError;

/// Type erased error that works for all kinds of store implementations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
	#[error("Entity not found")]
	NotFound(anyhow::Error),
	#[error("Connection error: {0}")]
	Connection(anyhow::Error),
	#[error("Database error: {0}")]
	Database(anyhow::Error),
	#[error("Unique constraint violation: {0}")]
	UniqueViolation(anyhow::Error),
	#[error("Foreign key violation: {0}")]
	ForeignKeyViolation(anyhow::Error),
	#[error("Other constraint violation: {0}")]
	OtherConstraintViolation(anyhow::Error),
	#[error("Encoding values: {0}")]
	Encode(anyhow::Error),
	#[error("Decoding values: {0}")]
	Decode(anyhow::Error),
	#[error("Migration error: {0}")]
	Migration(anyhow::Error),
	#[error("Timeout: {0}")]
	Timeout(anyhow::Error),
}

impl From<sqlx::Error> for StoreError {
	fn from(error: sqlx::Error) -> Self {
		use sqlx::Error::*;
		match error {
			Database(error) => error.into(),
			RowNotFound => Self::NotFound(error.into()),
			Encode(_) => Self::Encode(error.into()),
			Decode(_) => Self::Decode(error.into()),
			PoolTimedOut => Self::Timeout(error.into()),
			Migrate(error) => Self::Migration((*error).into()),
			other => Self::Database(other.into()),
		}
	}
}

impl From<Box<dyn sqlx::error::DatabaseError>> for StoreError {
	fn from(error: Box<dyn sqlx::error::DatabaseError>) -> Self {
		match error.kind() {
			ErrorKind::UniqueViolation => Self::UniqueViolation(error.into()),
			ErrorKind::ForeignKeyViolation => Self::ForeignKeyViolation(error.into()),
			ErrorKind::NotNullViolation | ErrorKind::CheckViolation => Self::OtherConstraintViolation(error.into()),
			_ => Self::Database(error.into()),
		}
	}
}

impl From<MigrateError> for StoreError {
	fn from(error: MigrateError) -> Self {
		Self::Migration(error.into())
	}
}

pub trait IntoStoreResult<Ok>: Sized {
	fn connection_error(self, context: &'static str) -> Result<Ok, StoreError>;
}

impl<Ok, Error> IntoStoreResult<Ok> for Result<Ok, Error>
where
	Error: std::error::Error + Send + Sync + 'static,
{
	fn connection_error(self, context: &'static str) -> Result<Ok, StoreError> {
		self.context(context).map_err(StoreError::Connection)
	}
}
