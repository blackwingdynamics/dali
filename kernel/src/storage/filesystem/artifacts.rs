//! Named durable artifact boundary for trust-store persistence.

use crate::drivers::StorageError;

use super::{read_root_file, write_root_file};

/// Fixed FAT-compatible names for the two trust-store slots and commit marker.
pub const TRUST_STORE_ACTIVE_FILE: &str = "DALI-ACT.BIN";
/// Fixed FAT-compatible name for the candidate trust-store artifact.
pub const TRUST_STORE_CANDIDATE_FILE: &str = "DALI-CAN.BIN";
/// Fixed FAT-compatible name for the durable trust-store commit marker.
pub const TRUST_STORE_COMMIT_FILE: &str = "DALI-CMT.BIN";

/// Kernel-owned durable trust-store artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustStoreArtifact {
    /// Last committed trust-store bundle.
    Active,
    /// Candidate bundle awaiting verification and commit.
    Candidate,
    /// Durable marker indicating that activation must be finalized.
    CommitMarker,
}

impl TrustStoreArtifact {
    /// Returns the fixed root-level FAT filename for this artifact.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Active => TRUST_STORE_ACTIVE_FILE,
            Self::Candidate => TRUST_STORE_CANDIDATE_FILE,
            Self::CommitMarker => TRUST_STORE_COMMIT_FILE,
        }
    }
}

/// Writes one named trust-store artifact and flushes its directory entry.
pub fn write_trust_store_artifact<D>(
    device: D,
    artifact: TrustStoreArtifact,
    contents: &[u8],
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    write_root_file(device, artifact.file_name(), contents)
}

/// Reads one named trust-store artifact into bounded caller-owned storage.
pub fn read_trust_store_artifact<D>(
    device: D,
    artifact: TrustStoreArtifact,
    output: &mut [u8],
) -> Result<usize, embedded_sdmmc::Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    read_root_file(device, artifact.file_name(), output)
}
