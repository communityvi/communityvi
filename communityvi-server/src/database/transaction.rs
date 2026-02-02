// use crate::database::Connection;
// use crate::database::error::DatabaseError;
// use static_assertions::assert_obj_safe;
// use std::ops::{Deref, DerefMut};
// use tracing::{error, warn};
//
// #[derive(Debug, thiserror::Error)]
// pub enum TransactionError<ApplicationError> {
// 	#[error("Database error: {0}")]
// 	Database(#[from] DatabaseError),
// 	#[error("{0}")]
// 	Application(ApplicationError),
// 	#[error("Transaction rolled back{}", rollback_reason_string(.0.as_deref()))]
// 	Rollback(Option<String>),
// 	#[error("Maximum attempts exceeded ({limit})")]
// 	AttemptsExhausted { limit: usize },
// }
//
// fn rollback_reason_string(reason: Option<&str>) -> String {
// 	match reason {
// 		Some(reason) => format!(": {}", reason),
// 		None => "".to_owned(),
// 	}
// }
//
// pub trait ConnectionTransactionExtension {
// 	fn run_in_transaction<Operation, OperationFuture, Output, ApplicationError>(
// 		self,
// 		operation: Operation,
// 	) -> impl Future<Output = Result<Output, TransactionError<ApplicationError>>> + Send
// 	where
// 		Output: Send,
// 		ApplicationError: Send,
// 		Operation: for<'connection> FnMut(&mut Transaction<'connection>) -> OperationFuture + Send,
// 		OperationFuture: Future<Output = Result<Output, TransactionError<ApplicationError>>> + Send;
// }
//
// impl<T> ConnectionTransactionExtension for T
// where
// 	T: Connection,
// {
// 	async fn run_in_transaction<Operation, OperationFuture, Output, ApplicationError>(
// 		mut self,
// 		mut operation: Operation,
// 	) -> Result<Output, TransactionError<ApplicationError>>
// 	where
// 		Output: Send,
// 		ApplicationError: Send,
// 		Operation: for<'connection> FnMut(&mut Transaction<'connection>) -> OperationFuture + Send,
// 		OperationFuture: Future<Output = Result<Output, TransactionError<ApplicationError>>> + Send,
// 	{
// 		const MAXIMUM_ATTEMPTS: usize = 5;
// 		for attempt in 1..=MAXIMUM_ATTEMPTS {
// 			let mut transaction = self.begin_transaction().await?;
// 			let serialization_error = match operation(&mut transaction).await {
// 				Ok(output) => {
// 					transaction.commit().await?;
// 					return Ok(output);
// 				}
// 				Err(TransactionError::Database(DatabaseError::TransactionSerialization(error))) => {
// 					transaction.rollback().await?;
// 					error
// 				}
// 				Err(error) => {
// 					transaction.rollback().await?;
// 					return Err(error);
// 				}
// 			};
//
// 			warn!(
// 				attempt,
// 				?serialization_error,
// 				"Transaction serialization error, retrying: {}",
// 				serialization_error
// 			);
// 		}
//
// 		Err(TransactionError::AttemptsExhausted {
// 			limit: MAXIMUM_ATTEMPTS,
// 		})
// 	}
// }
//
// pub struct Transaction<'connection> {
// 	transaction: Box<dyn DynTransaction<'connection> + 'connection>,
// }
//
// impl<'connection> Transaction<'connection> {
// 	pub fn new(transaction: impl DynTransaction<'connection> + 'connection) -> Self {
// 		Self {
// 			transaction: Box::new(transaction),
// 		}
// 	}
//
// 	async fn commit(self) -> Result<(), DatabaseError> {
// 		self.transaction.commit().await
// 	}
//
// 	async fn rollback(self) -> Result<(), DatabaseError> {
// 		self.transaction.rollback().await
// 	}
// }
//
// impl Deref for Transaction<'_> {
// 	type Target = dyn Connection;
//
// 	fn deref(&self) -> &Self::Target {
// 		self.transaction.as_connection()
// 	}
// }
//
// impl DerefMut for Transaction<'_> {
// 	fn deref_mut(&mut self) -> &mut Self::Target {
// 		self.transaction.as_mut_connection()
// 	}
// }
//
// #[async_trait::async_trait]
// pub trait DynTransaction<'connection>: 'connection + Send {
// 	fn type_name(&self) -> &'static str {
// 		std::any::type_name::<Self>()
// 	}
//
// 	fn as_connection(&self) -> &dyn Connection;
// 	fn as_mut_connection(&mut self) -> &mut dyn Connection;
//
// 	async fn commit(self: Box<Self>) -> Result<(), DatabaseError>;
// 	async fn rollback(self: Box<Self>) -> Result<(), DatabaseError>;
// }
//
// assert_obj_safe!(DynTransaction);
