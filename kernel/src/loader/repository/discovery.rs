//! Board-agnostic cartridge discovery through Binary Metadata v2 targets.

use dali_metadata::{CartridgeId, TargetCartridge};

use crate::storage::repository::RepositoryStreamStorage;

use super::streaming::{
    MAX_STREAMING_TARGET_RECORD_BYTES, STREAMING_METADATA_CHUNK_BYTES,
    StreamingTargetSelectionError, select_binary_target,
};

/// Errors returned while discovering one cartridge from repository metadata.
#[derive(Debug)]
pub enum CartridgeDiscoveryError<E> {
    /// The repository adapter failed while producing a stream.
    Storage(E),
    /// Binary Metadata v2 targets were malformed or exceeded the bounded selector.
    InvalidTargets,
    /// The requested cartridge ID was not declared by targets metadata.
    MissingCartridge,
}

/// Caller-owned buffers required by the bounded cartridge discovery pass.
pub struct CartridgeDiscoveryBuffers {
    /// Transport chunk used for all repository reads.
    pub chunk: [u8; STREAMING_METADATA_CHUNK_BYTES],
    /// Maximum selected target record capacity, kept as a named contract value.
    pub record_capacity: usize,
}

impl CartridgeDiscoveryBuffers {
    /// Creates the fixed discovery workspace.
    pub const fn new() -> Self {
        Self {
            chunk: [0; STREAMING_METADATA_CHUNK_BYTES],
            record_capacity: MAX_STREAMING_TARGET_RECORD_BYTES,
        }
    }
}

impl Default for CartridgeDiscoveryBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovers one cartridge without retaining the complete targets document.
pub fn discover_cartridge<S>(
    storage: &mut S,
    cartridge_id: CartridgeId,
    buffers: &mut CartridgeDiscoveryBuffers,
) -> Result<TargetCartridge, CartridgeDiscoveryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let target = select_binary_target(storage, cartridge_id, &mut buffers.chunk).map_err(
        |error| match error {
            StreamingTargetSelectionError::Storage(error) => {
                CartridgeDiscoveryError::Storage(error)
            }
            StreamingTargetSelectionError::Parse(_) => CartridgeDiscoveryError::InvalidTargets,
            StreamingTargetSelectionError::Verification => CartridgeDiscoveryError::InvalidTargets,
        },
    )?;
    target.ok_or(CartridgeDiscoveryError::MissingCartridge)
}
