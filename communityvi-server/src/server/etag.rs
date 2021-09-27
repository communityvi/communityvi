use include_dir::{Dir, File};
use rweb::hyper::header::HeaderValue;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, LowerHex};
use std::hash::Hasher;
use std::path::Path;

pub struct ETags {
	etags_by_path: HashMap<&'static Path, ETag>,
}

impl ETags {
	pub fn get(&self, path: &Path) -> Option<ETag> {
		self.etags_by_path.get(path).copied()
	}

	fn add_directory(&mut self, directory: &Dir<'static>) {
		for file in directory.files() {
			self.add_file(file);
		}

		for directory in directory.dirs() {
			self.add_directory(directory);
		}
	}

	fn add_file(&mut self, file: &File<'static>) {
		let etag = ETag::from(file);

		self.etags_by_path.insert(file.path(), etag);
	}
}

impl From<&Dir<'static>> for ETags {
	fn from(directory: &Dir<'static>) -> Self {
		let mut etags = ETags {
			etags_by_path: Default::default(),
		};
		etags.add_directory(directory);

		etags
	}
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ETag {
	hash: u64,
}

impl PartialEq<HeaderValue> for ETag {
	fn eq(&self, header: &HeaderValue) -> bool {
		self.to_string().as_bytes() == header.as_bytes()
	}
}

impl PartialEq<ETag> for HeaderValue {
	fn eq(&self, other: &ETag) -> bool {
		other.eq(self)
	}
}

impl From<ETag> for HeaderValue {
	fn from(etag: ETag) -> Self {
		HeaderValue::from_bytes(etag.to_string().as_bytes()).unwrap()
	}
}

impl From<&File<'_>> for ETag {
	fn from(file: &File<'_>) -> Self {
		let mut hasher = DefaultHasher::new();
		hasher.write(file.contents());

		Self { hash: hasher.finish() }
	}
}

impl Display for ETag {
	fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
		LowerHex::fmt(&self.hash, formatter)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use include_dir::include_dir;

	const BUNDLED_TEST_FILES: Dir = include_dir!("test/bundled_files");

	#[test]
	fn etag_should_stay_the_same_for_files_with_the_same_content() {
		let file = BUNDLED_TEST_FILES.get_file("test").unwrap();

		let etag1 = ETag::from(&file);
		let etag2 = ETag::from(&file);

		assert_eq!(etag1, etag2);
	}

	#[test]
	fn etag_should_be_different_for_files_with_different_content() {
		let file1 = BUNDLED_TEST_FILES.get_file("test").unwrap();
		let file2 = BUNDLED_TEST_FILES.get_file("index.html").unwrap();

		let etag1 = ETag::from(&file1);
		let etag2 = ETag::from(&file2);

		assert_ne!(etag1, etag2);
	}

	#[test]
	fn etags_can_be_created_from_a_directory() {
		let etags = ETags::from(&BUNDLED_TEST_FILES);

		let test = BUNDLED_TEST_FILES.get_file("test").unwrap();
		let test_etag = etags.get("test".as_ref()).unwrap();
		assert_eq!(ETag::from(&test), test_etag);

		let index = BUNDLED_TEST_FILES.get_file("index.html").unwrap();
		let index_etag = etags.get("index.html".as_ref()).unwrap();
		assert_eq!(ETag::from(&index), index_etag);

		let about = BUNDLED_TEST_FILES.get_file("about/index.html").unwrap();
		let about_etag = etags.get("about/index.html".as_ref()).unwrap();
		assert_eq!(ETag::from(&about), about_etag);
	}

	#[test]
	fn etag_should_not_exist_for_nonexistent_file() {
		let etags = ETags::from(&BUNDLED_TEST_FILES);

		let nonexistent_etag = etags.get("nonexistent".as_ref());

		assert!(nonexistent_etag.is_none())
	}

	#[test]
	fn etag_can_be_fetched_with_fuzzy_path() {
		let etags = ETags::from(&BUNDLED_TEST_FILES);

		let no_trailing_slash = etags.get("about/index.html".as_ref()).unwrap();
		let trailing_slash = etags.get("about/index.html/".as_ref()).unwrap();
		let double_slash = etags.get("about//index.html".as_ref()).unwrap();

		assert_eq!(no_trailing_slash, trailing_slash);
		assert_eq!(no_trailing_slash, double_slash);
		assert_eq!(double_slash, trailing_slash);
	}
}
