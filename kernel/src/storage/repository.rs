//! Board-agnostic repository metadata and package read contract.

/// Logical metadata document requested by the kernel loader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryDocument<'a> {
    /// Signed repository bundle manifest at the filesystem root.
    Bundle,
    /// Root trust policy.
    Root,
    /// Freshness reference to a snapshot.
    Timestamp,
    /// Consistent metadata references.
    Snapshot,
    /// Package target records.
    Targets,
    /// Explicit revocation records.
    Revocations,
    /// One developer delegation selected by metadata.
    Delegation(&'a str),
}

/// Width of a SHA-256 package digest in the repository contract.
pub const REPOSITORY_DIGEST_BYTES: usize = 32;

/// Fixed-size package digest used for content-addressed repository reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepositoryPackageDigest(pub [u8; REPOSITORY_DIGEST_BYTES]);

/// Chunk-streaming repository source consumed by the board-agnostic loader.
///
/// The contract deliberately exposes no whole-document read operation. A
/// storage adapter must deliver bytes through the caller-owned chunk, so the
/// loader can choose its RAM budget independently from the filesystem backend.
pub trait RepositoryStreamStorage {
    /// Adapter-specific storage failure type.
    type Error;

    /// Reads one metadata document in caller-selected bounded chunks.
    fn stream_metadata<F>(
        &mut self,
        document: RepositoryDocument<'_>,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>;

    /// Reads one package in caller-selected bounded chunks.
    fn stream_package<F>(
        &mut self,
        digest: RepositoryPackageDigest,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>;
}
