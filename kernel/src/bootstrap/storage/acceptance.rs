//! Feature-gated hardware acceptance checks for durable storage artifacts.

use crate::{
    drivers::{
        BlockReader, BlockTransportFlush, BlockWriter, FlushableBlockDevice, StorageError,
        WritableBlockDeviceAdapter,
    },
    storage::filesystem,
};

const ARTIFACT_TEST_LENGTH: usize = 32;
const ACTIVE_TEST_BYTE: u8 = 0xA5;
const CANDIDATE_TEST_BYTE: u8 = 0x5A;
const COMMIT_TEST_BYTE: u8 = 0xC3;

/// Identifies the durable artifact operation that failed during acceptance.
pub enum ArtifactTestError {
    /// The active trust-store artifact could not be written.
    ActiveWrite(embedded_sdmmc::Error<StorageError>),
    /// The active trust-store artifact could not be read back.
    ActiveRead(embedded_sdmmc::Error<StorageError>),
    /// The active trust-store artifact could not be flushed.
    ActiveFlush(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be written.
    CandidateWrite(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be read back.
    CandidateRead(embedded_sdmmc::Error<StorageError>),
    /// The candidate trust-store artifact could not be flushed.
    CandidateFlush(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be written.
    CommitWrite(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be read back.
    CommitRead(embedded_sdmmc::Error<StorageError>),
    /// The commit marker could not be flushed.
    CommitFlush(embedded_sdmmc::Error<StorageError>),
    /// Read-back data did not match the written bytes.
    ReadBackMismatch(filesystem::TrustStoreArtifact),
}

impl core::fmt::Debug for ArtifactTestError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ActiveWrite(error) => formatter.debug_tuple("ActiveWrite").field(error).finish(),
            Self::ActiveRead(error) => formatter.debug_tuple("ActiveRead").field(error).finish(),
            Self::ActiveFlush(error) => formatter.debug_tuple("ActiveFlush").field(error).finish(),
            Self::CandidateWrite(error) => formatter
                .debug_tuple("CandidateWrite")
                .field(error)
                .finish(),
            Self::CandidateRead(error) => {
                formatter.debug_tuple("CandidateRead").field(error).finish()
            }
            Self::CandidateFlush(error) => formatter
                .debug_tuple("CandidateFlush")
                .field(error)
                .finish(),
            Self::CommitWrite(error) => formatter.debug_tuple("CommitWrite").field(error).finish(),
            Self::CommitRead(error) => formatter.debug_tuple("CommitRead").field(error).finish(),
            Self::CommitFlush(error) => formatter.debug_tuple("CommitFlush").field(error).finish(),
            Self::ReadBackMismatch(artifact) => formatter
                .debug_tuple("ReadBackMismatch")
                .field(artifact)
                .finish(),
        }
    }
}

/// Verifies persistence through the real filesystem boundary.
///
/// This acceptance-only path overwrites the three reserved artifact names and
/// must therefore be enabled only on a test card.
pub fn verify_trust_store_artifacts<R>(
    device: &WritableBlockDeviceAdapter<R>,
) -> Result<(), ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    let active = [ACTIVE_TEST_BYTE; ARTIFACT_TEST_LENGTH];
    let candidate = [CANDIDATE_TEST_BYTE; ARTIFACT_TEST_LENGTH];
    let commit = [COMMIT_TEST_BYTE; ARTIFACT_TEST_LENGTH];

    verify_artifact(device, filesystem::TrustStoreArtifact::Active, &active)?;
    verify_artifact(
        device,
        filesystem::TrustStoreArtifact::Candidate,
        &candidate,
    )?;
    verify_artifact(
        device,
        filesystem::TrustStoreArtifact::CommitMarker,
        &commit,
    )
}

fn verify_artifact<R>(
    device: &WritableBlockDeviceAdapter<R>,
    artifact: filesystem::TrustStoreArtifact,
    expected: &[u8; ARTIFACT_TEST_LENGTH],
) -> Result<(), ArtifactTestError>
where
    R: BlockReader + BlockWriter + BlockTransportFlush<Error = StorageError>,
{
    if let Err(error) = filesystem::write_trust_store_artifact(device, artifact, expected) {
        return Err(write_error(artifact, error));
    }
    if let Err(error) = device.flush().map_err(embedded_sdmmc::Error::DeviceError) {
        return Err(flush_error(artifact, error));
    }
    let mut actual = [0; ARTIFACT_TEST_LENGTH];
    let length = match filesystem::read_trust_store_artifact(device, artifact, &mut actual) {
        Ok(length) => length,
        Err(error) => return Err(read_error(artifact, error)),
    };
    if length != expected.len() || actual != *expected {
        return Err(ArtifactTestError::ReadBackMismatch(artifact));
    }
    Ok(())
}

fn write_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveWrite(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateWrite(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitWrite(error),
    }
}

fn read_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveRead(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateRead(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitRead(error),
    }
}

fn flush_error(
    artifact: filesystem::TrustStoreArtifact,
    error: embedded_sdmmc::Error<StorageError>,
) -> ArtifactTestError {
    match artifact {
        filesystem::TrustStoreArtifact::Active => ArtifactTestError::ActiveFlush(error),
        filesystem::TrustStoreArtifact::Candidate => ArtifactTestError::CandidateFlush(error),
        filesystem::TrustStoreArtifact::CommitMarker => ArtifactTestError::CommitFlush(error),
    }
}
