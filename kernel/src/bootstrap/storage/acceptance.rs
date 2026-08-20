//! Feature-gated hardware acceptance checks for durable storage artifacts.

use crate::{
    drivers::{BlockReader, BlockWriter, StorageError, WritableBlockDeviceAdapter},
    storage::filesystem,
};

const ARTIFACT_TEST_LENGTH: usize = 32;
const ACTIVE_TEST_BYTE: u8 = 0xA5;
const CANDIDATE_TEST_BYTE: u8 = 0x5A;
const COMMIT_TEST_BYTE: u8 = 0xC3;

/// Verifies persistence through the real filesystem boundary.
///
/// This acceptance-only path overwrites the three reserved artifact names and
/// must therefore be enabled only on a test card.
pub fn verify_trust_store_artifacts<R>(
    device: &WritableBlockDeviceAdapter<R>,
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    R: BlockReader + BlockWriter,
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
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    R: BlockReader + BlockWriter,
{
    filesystem::write_trust_store_artifact(device, artifact, expected)?;
    let mut actual = [0; ARTIFACT_TEST_LENGTH];
    let length = filesystem::read_trust_store_artifact(device, artifact, &mut actual)?;
    if length != expected.len() || actual != *expected {
        return Err(embedded_sdmmc::Error::InvalidOffset);
    }
    Ok(())
}
