use crate::database::Connection;
use crate::database::error::DatabaseError;
use async_trait::async_trait;
use futures_util::future::BoxFuture;
use static_assertions::assert_obj_safe;
use std::future::Future;
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

pub trait ConnectionTransactionExtension {
	fn run_in_transaction<Operation, Output, ApplicationError>(
		&mut self,
		operation: Operation,
	) -> impl Future<Output = Result<Output, TransactionError<ApplicationError>>> + Send
	where
		Output: Send,
		ApplicationError: Send,
		Operation: for<'tx> FnMut(&'tx dyn Transaction) -> BoxFuture<'tx, Result<Output, TransactionError<ApplicationError>>>
			+ Send;
}

impl<C: Connection + ?Sized> ConnectionTransactionExtension for C {
	async fn run_in_transaction<Operation, Output, ApplicationError>(
		&mut self,
		mut operation: Operation,
	) -> Result<Output, TransactionError<ApplicationError>>
	where
		Output: Send,
		ApplicationError: Send,
		Operation: for<'tx> FnMut(&'tx dyn Transaction) -> BoxFuture<'tx, Result<Output, TransactionError<ApplicationError>>>
			+ Send,
	{
		const MAXIMUM_ATTEMPTS: usize = 5;
		for attempt in 1..=MAXIMUM_ATTEMPTS {
			let transaction = self.begin_transaction().await?;
			let serialization_error = match operation(transaction.as_ref()).await {
				Ok(output) => {
					transaction.commit().await?;
					return Ok(output);
				}
				Err(TransactionError::Database(DatabaseError::TransactionSerialization(error))) => {
					transaction.rollback().await?;
					error
				}
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
}

#[async_trait]
pub trait Transaction: Send + Sync {
	fn type_name(&self) -> &'static str {
		std::any::type_name::<Self>()
	}

	fn as_connection(&self) -> &dyn Connection;

	async fn commit(self: Box<Self>) -> Result<(), DatabaseError>;
	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError>;
}

assert_obj_safe!(Transaction);
