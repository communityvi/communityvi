use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use thiserror::Error;
use unicode_skeleton::UnicodeSkeleton;

#[derive(Default)]
pub struct UserRepository {
	users: HashMap<String, User>,
}

impl UserRepository {
	pub fn create_user(&mut self, name: &str) -> Result<User, UserCreationError> {
		if name.trim().is_empty() {
			return Err(UserCreationError::NameEmpty);
		}

		let normalized_name = normalized_name(name);

		const MAX_NAME_LENGTH: usize = 256;
		if name.len() > MAX_NAME_LENGTH {
			return Err(UserCreationError::NameTooLong);
		}

		use Entry::*;
		let Vacant(entry) = self.users.entry(normalized_name) else {
			return Err(UserCreationError::NameAlreadyInUse);
		};

		let user = User { name: name.to_owned() };
		entry.insert(user.clone());
		Ok(user)
	}

	pub fn remove(&mut self, user: &User) {
		let normalized_name = normalized_name(user.name());
		self.users.remove(&normalized_name);
	}
}

/// Ensure that unicode characters get correctly decomposed,
/// normalized and some homograph attacks are hindered, disregarding whitespace.
fn normalized_name(name: &str) -> String {
	name.split_whitespace()
		.flat_map(UnicodeSkeleton::skeleton_chars)
		.collect()
}

#[derive(Error, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // same prefix is OK, other error types will also live in here.
pub enum UserCreationError {
	#[error("Username was empty or whitespace-only.")]
	NameEmpty,
	#[error("Username is too long. (>256 bytes UTF-8)")]
	NameTooLong,
	#[error("Username is already in use.")]
	NameAlreadyInUse,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct User {
	/// The effective ID value of any user.
	name: String,
}

impl User {
	pub fn name(&self) -> &str {
		&self.name
	}
}

impl Borrow<str> for User {
	fn borrow(&self) -> &str {
		&self.name
	}
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod test {
	use super::*;

	#[test]
	fn should_normalize_unicode_strings() {
		assert_eq!(normalized_name("C\u{327}"), "C\u{326}");
		assert_eq!(normalized_name("é"), "e\u{301}");
		assert_eq!(normalized_name("\u{0C5}"), "A\u{30A}");
		assert_eq!(normalized_name("\u{212B}"), "A\u{30A}");
		assert_eq!(normalized_name("\u{391}"), "A");
		assert_eq!(normalized_name("\u{410}"), "A");
		assert_eq!(normalized_name("𝔭𝒶ỿ𝕡𝕒ℓ"), "paypal");
		assert_eq!(normalized_name("𝒶𝒷𝒸"), "abc");
		assert_eq!(normalized_name("ℝ𝓊𝓈𝓉"), "Rust");
		assert_eq!(normalized_name("аррӏе.com"), "appie.corn");
		assert_eq!(normalized_name("𝔭𝒶   ỿ𝕡𝕒		ℓ"), "paypal");
		assert_eq!(normalized_name("𝒶𝒷\r\n𝒸"), "abc");
		assert_eq!(normalized_name("ℝ		𝓊𝓈 𝓉"), "Rust");
		assert_eq!(normalized_name("арр    ӏе.	com"), "appie.corn");
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
		assert_eq!(normalized_name("аррӏе.com"), normalized_name("apple.com"));
	}

	#[test]
	fn should_not_create_with_empty_name() {
		let mut user_repository = UserRepository::default();

		let result = user_repository.create_user("");

		assert!(matches!(result, Err(UserCreationError::NameEmpty)));
	}

	#[test]
	fn should_not_create_with_blank_name() {
		let mut user_repository = UserRepository::default();

		let result = user_repository.create_user("  	 ");

		assert!(matches!(result, Err(UserCreationError::NameEmpty)));
	}

	#[test]
	fn should_not_create_two_users_with_the_same_name() {
		let mut user_repository = UserRepository::default();

		user_repository
			.create_user("Anorak  ")
			.expect("First create did not succeed!");
		let result = user_repository.create_user("   Anorak");

		assert!(matches!(result, Err(UserCreationError::NameAlreadyInUse)));
	}

	#[test]
	fn should_allow_creating_user_with_the_same_name_after_first_has_been_removed() {
		let mut user_repository = UserRepository::default();
		let name = "牧瀬 紅莉栖";

		let user = user_repository.create_user(name).expect("Failed to create user");
		user_repository.remove(&user);

		user_repository
			.create_user(name)
			.expect("Failed to create user with same name after first is gone");
	}

	#[test]
	fn should_allow_creating_users_with_name_no_longer_than_256_bytes() {
		let mut user_repository = UserRepository::default();
		let long_name = String::from_utf8(vec![0x41u8; 256]).unwrap();

		user_repository
			.create_user(&long_name)
			.expect("Failed to create user with name that has valid length");
	}

	#[test]
	fn should_not_allow_creating_users_with_name_longer_than_256_bytes() {
		let mut user_repository = UserRepository::default();
		let too_long_name = String::from_utf8(vec![0x41u8; 257]).unwrap();

		let result = user_repository.create_user(&too_long_name);

		assert!(matches!(result, Err(UserCreationError::NameTooLong)));
	}

	#[test]
	fn should_not_return_user_with_normalized_name() {
		const NAME: &str = "Thomas";

		let mut repository = UserRepository::default();

		let user = repository.create_user(NAME).expect("Failed to create user");

		assert_ne!(
			NAME,
			normalized_name(NAME),
			"This test only works if the normalization differs"
		);
		assert_eq!(NAME, user.name, "The created user should have had the original name.");
	}

	#[test]
	fn should_not_allow_user_with_homograph_name() {
		const NAME: &str = "Thomas";

		let mut repository = UserRepository::default();

		repository.create_user(NAME).expect("Failed to create user");

		let error = repository
			.create_user(&normalized_name(NAME))
			.expect_err("Should not have created user with homograph name");

		assert_eq!(UserCreationError::NameAlreadyInUse, error, "Incorrect error type");

		assert_ne!(
			NAME,
			normalized_name(NAME),
			"This test only works if the normalization differs"
		);
	}
}
