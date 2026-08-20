//! Board-agnostic repository metadata and package read contract.

/// Logical metadata document requested by the kernel loader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryDocument<'a> {
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

/// Logical repository source consumed by the board-agnostic kernel loader.
pub trait RepositoryStorage {
    /// Adapter-specific storage failure type.
    type Error;

    /// Reads one signed metadata document into a bounded caller-owned buffer.
    fn read_metadata(
        &mut self,
        document: RepositoryDocument<'_>,
        output: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Reads one content-addressed AMRN package into a bounded buffer.
    fn read_package(
        &mut self,
        digest: RepositoryPackageDigest,
        output: &mut [u8],
    ) -> Result<usize, Self::Error>;
}
