//! FAT32 adapter for board-agnostic repository and durable-storage traits.

use core::str;

use crate::storage::{
    durable::{DurableArtifact, DurableStorageAdapter},
    repository::{RepositoryDocument, RepositoryPackageDigest, RepositoryStreamStorage},
};

use super::{
    TrustStoreArtifact, read_trust_store_artifact, stream_repository_file,
    write_trust_store_artifact,
};

const PACKAGE_NAME_BYTES: usize = 64 + 5;
const DELEGATION_NAME_BYTES: usize = 64 + 5;

/// FAT32-backed logical repository storage.
///
/// The device is copied into each filesystem pass. On the F405 path this is
/// a reference to `WritableBlockDeviceAdapter`, so the adapter retains no
/// board or controller-specific type.
#[derive(Clone, Copy)]
pub struct FatRepositoryStorage<D> {
    device: D,
}

impl<D> FatRepositoryStorage<D> {
    /// Creates a repository adapter over an initialized FAT block device.
    pub const fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D> DurableStorageAdapter for FatRepositoryStorage<D>
where
    D: Copy + embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
{
    type Error = embedded_sdmmc::Error<crate::drivers::StorageError>;

    fn read_artifact(
        &mut self,
        artifact: DurableArtifact,
        output: &mut [u8],
    ) -> Result<usize, Self::Error> {
        read_trust_store_artifact(self.device, trust_store_artifact(artifact), output)
    }

    fn write_artifact(
        &mut self,
        artifact: DurableArtifact,
        contents: &[u8],
    ) -> Result<(), Self::Error> {
        write_trust_store_artifact(self.device, trust_store_artifact(artifact), contents)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<D> RepositoryStreamStorage for FatRepositoryStorage<D>
where
    D: Copy + embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
{
    type Error = embedded_sdmmc::Error<crate::drivers::StorageError>;

    fn stream_metadata<F>(
        &mut self,
        document: RepositoryDocument<'_>,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>,
    {
        match document {
            RepositoryDocument::Root => {
                stream_file(self.device, "metadata", None, "root.json", chunk, consumer)
            }
            RepositoryDocument::Timestamp => stream_file(
                self.device,
                "metadata",
                None,
                "timestamp.json",
                chunk,
                consumer,
            ),
            RepositoryDocument::Snapshot => stream_file(
                self.device,
                "metadata",
                None,
                "snapshot.json",
                chunk,
                consumer,
            ),
            RepositoryDocument::Targets => stream_file(
                self.device,
                "metadata",
                None,
                "targets.json",
                chunk,
                consumer,
            ),
            RepositoryDocument::Revocations => stream_file(
                self.device,
                "metadata",
                None,
                "revocations.json",
                chunk,
                consumer,
            ),
            RepositoryDocument::Delegation(identifier) => {
                let mut name = [0u8; DELEGATION_NAME_BYTES];
                let name = append_json(identifier, &mut name)?;
                stream_file(
                    self.device,
                    "metadata",
                    Some("delegations"),
                    name,
                    chunk,
                    consumer,
                )
            }
        }
    }

    fn stream_package<F>(
        &mut self,
        digest: RepositoryPackageDigest,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>,
    {
        let mut name = [0u8; PACKAGE_NAME_BYTES];
        let name = append_package_suffix(&digest.0, &mut name)?;
        stream_file(self.device, "packages", None, name, chunk, consumer)
    }
}

fn append_json<'a>(
    identifier: &str,
    output: &'a mut [u8; DELEGATION_NAME_BYTES],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    append_suffix(identifier.as_bytes(), b".json", output)
}

fn stream_file<D, F>(
    device: D,
    first_directory: &str,
    second_directory: Option<&str>,
    file_name: &str,
    chunk: &mut [u8],
    consumer: F,
) -> Result<u32, embedded_sdmmc::Error<crate::drivers::StorageError>>
where
    D: Copy + embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
    F: FnMut(&[u8]) -> Result<(), embedded_sdmmc::Error<crate::drivers::StorageError>>,
{
    stream_repository_file(
        device,
        first_directory,
        second_directory,
        file_name,
        chunk,
        consumer,
    )
}

fn append_package_suffix<'a>(
    digest: &[u8; 32],
    output: &'a mut [u8; PACKAGE_NAME_BYTES],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    let mut length = 0;
    for byte in digest {
        output[length] = hex_digit(byte >> 4);
        output[length + 1] = hex_digit(byte & 0x0F);
        length += 2;
    }
    append_suffix_at(length, b".amrn", output)
}

fn append_suffix<'a>(
    prefix: &[u8],
    suffix: &[u8],
    output: &'a mut [u8],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    if prefix.len() + suffix.len() > output.len() {
        return Err(embedded_sdmmc::Error::InvalidOffset);
    }
    output[..prefix.len()].copy_from_slice(prefix);
    append_suffix_at(prefix.len(), suffix, output)
}

fn append_suffix_at<'a>(
    offset: usize,
    suffix: &[u8],
    output: &'a mut [u8],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    let end = offset
        .checked_add(suffix.len())
        .ok_or(embedded_sdmmc::Error::InvalidOffset)?;
    if end > output.len() {
        return Err(embedded_sdmmc::Error::InvalidOffset);
    }
    output[offset..end].copy_from_slice(suffix);
    str::from_utf8(&output[..end]).map_err(|_| embedded_sdmmc::Error::InvalidOffset)
}

const fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + (value - 10),
    }
}

const fn trust_store_artifact(artifact: DurableArtifact) -> TrustStoreArtifact {
    match artifact {
        DurableArtifact::SlotA => TrustStoreArtifact::Active,
        DurableArtifact::SlotB => TrustStoreArtifact::Candidate,
        DurableArtifact::CommitJournal => TrustStoreArtifact::CommitMarker,
    }
}
