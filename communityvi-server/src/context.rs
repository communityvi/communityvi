use crate::configuration::Configuration;
use crate::error::CommunityviError;
use crate::reference_time::ReferenceTimer;
use crate::user::UserRepository;
use crate::utils::time_source::TimeSource;
use axum::extract::FromRef;
use base64::display::Base64Display;
use base64::prelude::BASE64_STANDARD;
use jsonwebtoken::{DecodingKey, EncodingKey};
use log::warn;
use parking_lot::Mutex;
use rand::Rng;
use std::sync::Arc;

#[derive(Clone, FromRef)]
pub struct ApplicationContext {
	pub configuration: Configuration,
	pub time_source: TimeSource,
	pub reference_timer: ReferenceTimer,
	pub user_repository: Arc<Mutex<UserRepository>>,
	jwt_encoding_key: EncodingKey,
	jwt_decoding_key: DecodingKey,
}

impl ApplicationContext {
	pub fn new(configuration: Configuration, time_source: TimeSource) -> Result<ApplicationContext, CommunityviError> {
		let jwt_keys = configuration.jwt_keys()?;
		let (encoding_key, decoding_key) = if let Some(keys) = jwt_keys {
			keys
		} else {
			let secret = rand::thread_rng().gen::<[u8; 32]>();
			warn!(
				"No JWT secret configured! Random secret: {}",
				Base64Display::new(&secret, &BASE64_STANDARD)
			);
			(EncodingKey::from_secret(&secret), DecodingKey::from_secret(&secret))
		};

		Ok(Self {
			configuration,
			time_source,
			reference_timer: Default::default(),
			user_repository: Default::default(),
			jwt_encoding_key: encoding_key,
			jwt_decoding_key: decoding_key,
		})
	}
}
