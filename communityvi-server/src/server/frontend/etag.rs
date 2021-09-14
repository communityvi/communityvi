use gotham::hyper::header::HeaderValue;
use include_dir::{Dir, File};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, LowerHex};
use std::hash::Hasher;
use std::path::{Path, PathBuf};

pub struct ETags {
	etags_by_path: HashMap<PathBuf, ETag>,
}

impl ETags {
	pub fn get(&self, path: &Path) -> Option<ETag> {
		self.etags_by_path.get(path).copied()
	}

	fn add_directory(&mut self, directory: &Dir) {
		for file in directory.files() {
			self.add_file(file);
		}

		for directory in directory.dirs() {
			self.add_directory(directory);
		}
	}

	fn add_file(&mut self, file: &File) {
		let path = file.path().to_path_buf();
		let etag = ETag::from(file);

		self.etags_by_path.insert(path, etag);
	}
}

impl From<&Dir<'_>> for ETags {
	fn from(directory: &Dir<'_>) -> Self {
		let mut etags = ETags {
			etags_by_path: Default::default(),
		};
		etags.add_directory(directory);

		etags
	}
}

#[derive(Copy, Clone, PartialEq, Eq)]
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
