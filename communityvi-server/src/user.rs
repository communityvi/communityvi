use crate::database::Connection;
use crate::database::error::DatabaseError;
use crate::types::uuid::Uuid;
use crate::user::model::User;
use crate::user::repository::UserRepository;
use std::sync::Arc;
use thiserror::Error;
use unicode_skeleton::UnicodeSkeleton;

pub mod model;
pub mod repository;

#[derive(Clone)]
pub struct UserService {
	repository: Arc<dyn UserRepository>,
}

impl UserService {
	pub fn new(repository: Arc<dyn UserRepository>) -> Self {
		Self { repository }
	}

	pub async fn create_user(&self, name: &str, connection: &mut dyn Connection) -> Result<User, UserCreationError> {
		if name.trim().is_empty() {
			return Err(UserCreationError::NameEmpty);
		}

		const MAX_NAME_LENGTH: usize = 256;
		if name.len() > MAX_NAME_LENGTH {
			return Err(UserCreationError::NameTooLong);
		}

		let user = self
			.repository
			.create(connection, name, &normalize_name(name))
			.await
			.map_err(|error| match error {
				DatabaseError::UniqueViolation(_) => UserCreationError::NameAlreadyInUse,
				other => other.into(),
			})?;

		Ok(user)
	}

	pub async fn remove(&self, user_uuid: Uuid, connection: &mut dyn Connection) -> Result<(), DatabaseError> {
		self.repository.remove(connection, user_uuid).await
	}
}

/// Ensure that unicode characters get correctly decomposed,
/// normalized and some homograph attacks are hindered, disregarding whitespace.
pub fn normalize_name(name: &str) -> String {
	name.split_whitespace()
		.flat_map(UnicodeSkeleton::skeleton_chars)
		.collect()
}

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)] // same prefix is OK, other error types will also live in here.
pub enum UserCreationError {
	#[error("Username was empty or whitespace-only.")]
	NameEmpty,
	#[error("Username is too long. (>256 bytes UTF-8)")]
	NameTooLong,
	#[error("Username is already in use.")]
	NameAlreadyInUse,
	#[error("Database error: {0}")]
	Database(#[from] DatabaseError),
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod test {
	use super::*;
	use crate::database::sqlite::test_utils::{connection, repository};

	#[test]
	fn should_normalize_unicode_strings() {
		assert_eq!(normalize_name("C\u{327}"), "C\u{326}");
		assert_eq!(normalize_name("é"), "e\u{301}");
		assert_eq!(normalize_name("\u{0C5}"), "A\u{30A}");
		assert_eq!(normalize_name("\u{212B}"), "A\u{30A}");
		assert_eq!(normalize_name("\u{391}"), "A");
		assert_eq!(normalize_name("\u{410}"), "A");
		assert_eq!(normalize_name("𝔭𝒶ỿ𝕡𝕒ℓ"), "paypal");
		assert_eq!(normalize_name("𝒶𝒷𝒸"), "abc");
		assert_eq!(normalize_name("ℝ𝓊𝓈𝓉"), "Rust");
		assert_eq!(normalize_name("аррӏе.com"), "appie.corn");
		assert_eq!(normalize_name("𝔭𝒶   ỿ𝕡𝕒		ℓ"), "paypal");
		assert_eq!(normalize_name("𝒶𝒷\r\n𝒸"), "abc");
		assert_eq!(normalize_name("ℝ		𝓊𝓈 𝓉"), "Rust");
		assert_eq!(normalize_name("арр    ӏе.	com"), "appie.corn");
	}

	#[test]
	#[ignore = "We don't currently prevent whole-script homographs, but we should!"]
	fn should_prevent_whole_script_homographs() {
		/*
		 * "Our IDN threat model specifically excludes whole-script homographs, because they can't
		 *  be detected programmatically and our "TLD whitelist" approach didn't scale in the face
		 *  of a large number of new TLDs. If you are buying a domain in a registry which does not
		 *  have proper anti-spoofing protections (like .com), it is sadly the responsibility of
		 *  domain owners to check for whole-script homographs and register them."
		 *  - https://bugzilla.mozilla.org/show_bug.cgi?id=1332714#c5 by Gervase Markham, 2017-01-25
		 */
		assert_eq!(normalize_name("аррӏе.com"), normalize_name("apple.com"));
	}

	#[tokio::test]
	async fn should_not_create_with_empty_name() {
		let user_service = user_service();
		let mut connection = connection().await;

		let result = user_service.create_user("", connection.as_mut()).await;

		assert!(matches!(result, Err(UserCreationError::NameEmpty)));
	}

	#[tokio::test]
	async fn should_not_create_with_blank_name() {
		let user_service = user_service();
		let mut connection = connection().await;

		let result = user_service.create_user("  	 ", connection.as_mut()).await;

		assert!(matches!(result, Err(UserCreationError::NameEmpty)));
	}

	#[tokio::test]
	async fn should_not_create_two_users_with_the_same_name() {
		let user_service = user_service();
		let mut connection = connection().await;

		user_service
			.create_user("Anorak  ", connection.as_mut())
			.await
			.expect("First create did not succeed!");
		let result = user_service.create_user("   Anorak", connection.as_mut()).await;

		assert!(matches!(result, Err(UserCreationError::NameAlreadyInUse)));
	}

	#[tokio::test]
	async fn should_allow_creating_user_with_the_same_name_after_first_has_been_removed() {
		let user_service = user_service();
		let mut connection = connection().await;
		let name = "牧瀬 紅莉栖";

		let user = user_service
			.create_user(name, connection.as_mut())
			.await
			.expect("Failed to create user");
		user_service
			.remove(user.uuid, connection.as_mut())
			.await
			.expect("Failed to remove user");

		user_service
			.create_user(name, connection.as_mut())
			.await
			.expect("Failed to create user with same name after first is gone");
	}

	#[tokio::test]
	async fn should_allow_creating_users_with_name_no_longer_than_256_bytes() {
		let user_service = user_service();
		let mut connection = connection().await;
		let long_name = String::from_utf8(vec![0x41u8; 256]).unwrap();

		user_service
			.create_user(&long_name, connection.as_mut())
			.await
			.expect("Failed to create user with name that has valid length");
	}

	#[tokio::test]
	async fn should_not_allow_creating_users_with_name_longer_than_256_bytes() {
		let user_service = user_service();
		let mut connection = connection().await;
		let too_long_name = String::from_utf8(vec![0x41u8; 257]).unwrap();

		let result = user_service.create_user(&too_long_name, connection.as_mut()).await;

		assert!(matches!(result, Err(UserCreationError::NameTooLong)));
	}

	#[tokio::test]
	async fn should_not_return_user_with_normalized_name() {
		const NAME: &str = "Thomas";

		let user_service = user_service();
		let mut connection = connection().await;

		let user = user_service
			.create_user(NAME, connection.as_mut())
			.await
			.expect("Failed to create user");

		assert_ne!(
			NAME,
			normalize_name(NAME),
			"This test only works if the normalization differs"
		);
		assert_eq!(NAME, user.name, "The created user should have had the original name.");
	}

	#[tokio::test]
	async fn should_not_allow_user_with_homograph_name() {
		const NAME: &str = "Thomas";

		let user_service = user_service();
		let mut connection = connection().await;

		user_service
			.create_user(NAME, connection.as_mut())
			.await
			.expect("Failed to create user");

		let error = user_service
			.create_user(&normalize_name(NAME), connection.as_mut())
			.await
			.expect_err("Should not have created user with homograph name");

		assert!(
			matches!(error, UserCreationError::NameAlreadyInUse),
			"Incorrect error type"
		);

		assert_ne!(
			NAME,
			normalize_name(NAME),
			"This test only works if the normalization differs"
		);
	}

	fn user_service() -> UserService {
		let repository = repository();
		UserService::new(repository)
	}
}
