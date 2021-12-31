use lazy_static::lazy_static;
use parking_lot::RwLock;
use rust_embed::{EmbeddedFile, Filenames, Metadata, RustEmbed};
use rust_embed5::{Filenames as Filenames5, RustEmbed as RustEmbed5};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

pub struct RustEmbedAdapter<T>(PhantomData<T>);

impl<T> RustEmbed for RustEmbedAdapter<T>
where
	T: RustEmbed5,
{
	fn get(file_path: &str) -> Option<EmbeddedFile> {
		T::get(file_path).map(|data| {
			let hash = cached_sha256(file_path.as_ref(), &data);
			EmbeddedFile {
				data,
				// NOTE: This relies on unstable API but we'll have to live with that for now
				// since there is no other way to adapt the old RustEmbed to the new one.
				metadata: Metadata::__rust_embed_new(hash, None),
			}
		})
	}

	fn iter() -> Filenames {
		#[cfg(not(debug_assertions))]
		{
			let Filenames5::Embedded(iterator) = T::iter();
			Filenames::Embedded(iterator)
		}

		#[cfg(debug_assertions)]
		{
			let Filenames5::Dynamic(iterator) = T::iter();
			Filenames::Dynamic(iterator)
		}
	}
}

fn cached_sha256(path: &Path, bytes: &[u8]) -> [u8; 32] {
	lazy_static! {
		static ref CACHE: RwLock<HashMap<PathBuf, [u8; 32]>> = RwLock::default();
	};

	{
		let cache = CACHE.read();
		if let Some(&hash) = cache.get(path) {
			return hash;
		}
	}

	*CACHE.write().entry(path.into()).or_insert_with(|| {
		let mut hasher = Sha256::default();
		hasher.update(bytes);
		hasher.finalize().into()
	})
}
