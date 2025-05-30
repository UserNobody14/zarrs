//! Zarr chunk key encodings. Includes a [default](default::DefaultChunkKeyEncoding) and [v2](v2::V2ChunkKeyEncoding) implementation.
//!
//! See <https://zarr-specs.readthedocs.io/en/latest/v3/chunk-key-encodings/index.html>.

pub mod default;
pub mod v2;

use std::sync::Arc;

pub use zarrs_metadata::ChunkKeySeparator;
pub use zarrs_metadata_ext::chunk_key_encoding::{
    default::DefaultChunkKeyEncodingConfiguration, v2::V2ChunkKeyEncodingConfiguration,
};

pub use default::DefaultChunkKeyEncoding;
pub use v2::V2ChunkKeyEncoding;
use zarrs_plugin::PluginUnsupportedError;

use crate::{
    metadata::v3::MetadataV3,
    plugin::{Plugin, PluginCreateError},
    storage::StoreKey,
};

use derive_more::{Deref, From};

/// A chunk key encoding.
#[derive(Debug, Clone, From, Deref)]
pub struct ChunkKeyEncoding(Arc<dyn ChunkKeyEncodingTraits>);

/// A chunk key encoding plugin.
#[derive(derive_more::Deref)]
pub struct ChunkKeyEncodingPlugin(Plugin<ChunkKeyEncoding, MetadataV3>);
inventory::collect!(ChunkKeyEncodingPlugin);

impl ChunkKeyEncodingPlugin {
    /// Create a new [`ChunkKeyEncodingPlugin`].
    pub const fn new(
        identifier: &'static str,
        match_name_fn: fn(name: &str) -> bool,
        create_fn: fn(metadata: &MetadataV3) -> Result<ChunkKeyEncoding, PluginCreateError>,
    ) -> Self {
        Self(Plugin::new(identifier, match_name_fn, create_fn))
    }
}

impl ChunkKeyEncoding {
    /// Create a chunk key encoding.
    pub fn new<T: ChunkKeyEncodingTraits + 'static>(chunk_key_encoding: T) -> Self {
        let chunk_key_encoding: Arc<dyn ChunkKeyEncodingTraits> = Arc::new(chunk_key_encoding);
        chunk_key_encoding.into()
    }

    /// Create a chunk key encoding from metadata.
    ///
    /// # Errors
    ///
    /// Returns [`PluginCreateError`] if the metadata is invalid or not associated with a registered chunk key encoding plugin.
    pub fn from_metadata(metadata: &MetadataV3) -> Result<Self, PluginCreateError> {
        for plugin in inventory::iter::<ChunkKeyEncodingPlugin> {
            if plugin.match_name(metadata.name()) {
                return plugin.create(metadata);
            }
        }
        #[cfg(miri)]
        {
            // Inventory does not work in miri, so manually handle all known chunk key encodings
            match metadata.name() {
                chunk_key_encoding::DEFAULT => {
                    return default::create_chunk_key_encoding_default(metadata);
                }
                chunk_key_encoding::V2 => {
                    return v2::create_chunk_key_encoding_v2(metadata);
                }
                _ => {}
            }
        }
        Err(PluginUnsupportedError::new(
            metadata.name().to_string(),
            "chunk key encoding".to_string(),
        )
        .into())
    }
}

impl<T> From<T> for ChunkKeyEncoding
where
    T: ChunkKeyEncodingTraits + 'static,
{
    fn from(chunk_key_encoding: T) -> Self {
        Self::new(chunk_key_encoding)
    }
}

/// Chunk key encoding traits.
pub trait ChunkKeyEncodingTraits: core::fmt::Debug + Send + Sync {
    /// Create the metadata of this chunk key encoding.
    fn create_metadata(&self) -> MetadataV3;

    /// Encode chunk grid indices (grid cell coordinates) into a store key.
    fn encode(&self, chunk_grid_indices: &[u64]) -> StoreKey;
}
