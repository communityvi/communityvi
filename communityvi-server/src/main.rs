use crate::commandline::Commandline;
use crate::error::CommunityviError;
use clap::Parser;

mod chat_message;
mod commandline;
mod configuration;
mod connection;
mod context;
mod database;
mod error;
mod lifecycle;
mod message;
mod reference_time;
mod room;
mod server;
#[cfg(test)]
mod server_tests;
mod user;
mod utils;

#[tokio::main]
async fn main() -> Result<(), CommunityviError> {
	let commandline = Commandline::parse();
	commandline.run().await
}
