use crate::database::Connection;
use crate::database::error::DatabaseError;
use static_assertions::assert_obj_safe;
use tracing::warn;

#[derive(Debug, thiserror::Error)]
pub enum TransactionError<ApplicationError> {
	#[error("Database error: {0}")]
	Database(#[from] DatabaseError),
	#[error("{0}")]
	Application(ApplicationError),
	#[error("Transaction rolled back{}", rollback_reason_string(.0.as_deref()))]
	Rollback(Option<String>),
	#[error("Maximum attempts exceeded ({limit})")]
	AttemptsExhausted { limit: usize },
}

fn rollback_reason_string(reason: Option<&str>) -> String {
	match reason {
		Some(reason) => format!(": {}", reason),
		None => "".to_owned(),
	}
}

// NOTE: Requires the underlying type to automatically roll back on drop.
#[async_trait::async_trait]
pub trait Transaction<'connection> {
	fn type_name(&self) -> &'static str {
		std::any::type_name::<Self>()
	}

	fn as_mut_connection(&mut self) -> &mut dyn Connection;

	async fn commit(self: Box<Self>) -> Result<(), DatabaseError>;
	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError>;
}

assert_obj_safe!(Transaction);

pub async fn run_in_transaction<Operation, Output, ApplicationError>(
	mut transaction: Box<dyn Transaction<'_>>,
	mut operation: Operation,
) -> Result<Output, TransactionError<ApplicationError>>
where
	Operation: AsyncFnMut(&mut dyn Connection) -> Result<Output, TransactionError<ApplicationError>>,
{
	const MAXIMUM_ATTEMPTS: usize = 5;
	for attempt in 1..=MAXIMUM_ATTEMPTS {
		let serialization_error = match operation(transaction.as_mut_connection()).await {
			Ok(output) => {
				transaction.commit().await?;
				return Ok(output);
			}
			Err(TransactionError::Database(DatabaseError::TransactionSerialization(error))) => error,
			Err(error) => {
				transaction.rollback().await?;
				return Err(error);
			}
		};

		warn!(
			attempt,
			?serialization_error,
			"Transaction serialization error, retrying: {}",
			serialization_error
		);
	}

	Err(TransactionError::AttemptsExhausted {
		limit: MAXIMUM_ATTEMPTS,
	})
}
