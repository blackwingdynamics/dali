//! Board-agnostic package discovery through Binary Metadata v2 targets.

use dali_metadata::{PackageId, TargetPackage};

use crate::storage::repository::RepositoryStreamStorage;

use super::streaming::{
    MAX_STREAMING_TARGET_RECORD_BYTES, STREAMING_METADATA_CHUNK_BYTES,
    StreamingTargetSelectionError, select_binary_target,
};

/// Errors returned while discovering one package from repository metadata.
#[derive(Debug)]
pub enum PackageDiscoveryError<E> {
    /// The repository adapter failed while producing a stream.
    Storage(E),
    /// Binary Metadata v2 targets were malformed or exceeded the bounded selector.
    InvalidTargets,
    /// The requested package ID was not declared by targets metadata.
    MissingPackage,
}

/// Caller-owned buffers required by the bounded package discovery pass.
pub struct PackageDiscoveryBuffers {
    /// Transport chunk used for all repository reads.
    pub chunk: [u8; STREAMING_METADATA_CHUNK_BYTES],
    /// Maximum selected target record capacity, kept as a named contract value.
    pub record_capacity: usize,
}

impl PackageDiscoveryBuffers {
    /// Creates the fixed discovery workspace.
    pub const fn new() -> Self {
        Self {
            chunk: [0; STREAMING_METADATA_CHUNK_BYTES],
            record_capacity: MAX_STREAMING_TARGET_RECORD_BYTES,
        }
    }
}

impl Default for PackageDiscoveryBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovers one package without retaining the complete targets document.
pub fn discover_package<S>(
    storage: &mut S,
    package_id: PackageId,
    buffers: &mut PackageDiscoveryBuffers,
) -> Result<TargetPackage, PackageDiscoveryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let target = select_binary_target(storage, package_id, &mut buffers.chunk).map_err(
        |error| match error {
            StreamingTargetSelectionError::Storage(error) => PackageDiscoveryError::Storage(error),
            StreamingTargetSelectionError::Parse(_) => PackageDiscoveryError::InvalidTargets,
        },
    )?;
    target.ok_or(PackageDiscoveryError::MissingPackage)
}
