//! Hardware-neutral contracts for staged artifact installation.

use crate::storage::StorageError;

/// A persistent writer for an uncommitted artifact staging area.
pub trait ArtifactStager {
    /// Returns the maximum artifact length accepted by the staging area.
    fn staging_capacity(&self) -> Result<u32, StorageError>;

    /// Invalidates any previous candidate and starts a bounded new transfer.
    ///
    /// The candidate must not become visible to the boot source until
    /// [`Self::commit_staging`] succeeds.
    fn begin_staging(&mut self, length: u32) -> Result<(), StorageError>;

    /// Writes one complete caller-owned chunk into the candidate artifact.
    ///
    /// Implementations must reject ranges outside the candidate length and
    /// must not publish a partially written candidate as active.
    fn write_staging(&mut self, offset: u32, bytes: &[u8]) -> Result<(), StorageError>;

    /// Publishes the completely validated candidate as the active artifact.
    ///
    /// The caller must validate the staged bytes through the normal AMRN
    /// loader contract before invoking this operation.
    fn commit_staging(&mut self) -> Result<(), StorageError>;

    /// Discards the current candidate without changing the active artifact.
    fn abort_staging(&mut self) -> Result<(), StorageError>;
}
