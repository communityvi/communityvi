use deadpool::managed::{Manager, Metrics, Object, Pool, RecycleError, RecycleResult};

pub type TursoPool = Pool<TursoManager, Object<TursoManager>>;

pub struct TursoManager {
	database: turso::Database,
}

impl TursoManager {
	pub fn new(database: turso::Database) -> Self {
		Self { database }
	}
}

impl Manager for TursoManager {
	type Type = turso::Connection;
	type Error = turso::Error;

	async fn create(&self) -> Result<Self::Type, Self::Error> {
		self.database.connect()
	}

	async fn recycle(&self, connection: &mut Self::Type, _metrics: &Metrics) -> RecycleResult<Self::Error> {
		let mut rows = connection.query("SELECT 1", ()).await?;
		let Some(first) = rows.next().await? else {
			return Err(RecycleError::Message("Ping query returned zero results".into()));
		};

		let value = first
			.get::<i64>(0)
			.map_err(|error| RecycleError::Backend(error.into()))?;
		if value != 1 {
			return Err(RecycleError::Message("Ping query returned unexpected result".into()));
		}

		let None = rows.next().await? else {
			return Err(RecycleError::Message("Ping query returned more than one result".into()));
		};

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use deadpool::managed::PoolBuilder;

	#[tokio::test]
	async fn get_connection() {
		let database = turso::Builder::new_local(":memory:")
			.build()
			.await
			.expect("Failed to build turso database");
		let manager = TursoManager::new(database);
		let pool = TursoPool::builder(manager).build().expect("Failed to build turso pool");

		let connection = pool.get().await.expect("Failed to get connection from pool");

		let rows = connection
			.query("SELECT 1 + 1", ())
			.await
			.expect("Failed to execute query")
			.next()
			.await
			.expect("Failed to fetch row")
			.expect("Expected at least one row");
		let number = rows.get::<i64>(0).expect("Failed to get value from row");
		assert_eq!(2, number);
	}
}
