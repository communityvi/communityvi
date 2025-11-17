/// Type erased error that works for all kinds of store implementations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
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
	#[error("Transaction serialization error: {0}")]
	TransactionSerialization(anyhow::Error),
	#[error("Encoding values: {0}")]
	Encode(anyhow::Error),
	#[error("Decoding values: {0}")]
	Decode(anyhow::Error),
	#[error("Migration error: {0}")]
	Migration(anyhow::Error),
	#[error("Timeout: {0}")]
	Timeout(anyhow::Error),
	#[error("Repository and connection are for different databases: {0}")]
	DatabaseMismatch(anyhow::Error),
}
