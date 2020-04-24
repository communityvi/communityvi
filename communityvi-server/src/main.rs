use crate::commandline::Commandline;
use crate::error::CommunityviError;
use structopt::StructOpt;

mod commandline;
mod configuration;
mod connection;
mod error;
mod lifecycle;
mod message;
mod room;
mod server;
#[cfg(test)]
mod server_tests;
mod utils;

#[tokio::main]
async fn main() -> Result<(), CommunityviError> {
	let commandline = Commandline::from_args();
	commandline.run().await
}
