use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::database::libsql::libsql_connection;
use rust_embed::RustEmbed;
use std::collections::BTreeMap;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/migrations"]
struct Migrations;

pub async fn run_migrations(connection: &dyn Connection) -> Result<(), DatabaseError> {
	let connection = libsql_connection(connection)?;
	let migrations = Migrations::iter()
		.filter_map(|file_name| Migrations::get(&file_name).map(|file| (file_name, file)))
		.collect::<BTreeMap<_, _>>();

	let transaction = connection.transaction().await?;
	for migration in migrations.values() {
		// TODO: Track already applied migrations
		let sql = str::from_utf8(migration.data.as_ref()).map_err(|error| DatabaseError::Migration(error.into()))?;
		transaction.execute_batch(sql).await?;
	}
	transaction.commit().await?;

	Ok(())
}
