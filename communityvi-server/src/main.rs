#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unseparated_literal_suffix)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::used_underscore_binding)]
use crate::commandline::Commandline;
use crate::error::CommunityviError;
use clap::Clap;

mod commandline;
mod configuration;
mod connection;
mod context;
mod error;
#[cfg(test)]
mod gotham_server_tests;
mod lifecycle;
mod message;
mod room;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<(), CommunityviError> {
	let commandline = Commandline::parse();
	commandline.run().await
}
