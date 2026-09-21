//! FAT32 adapter for board-agnostic repository and durable-storage traits.

use core::str;

use embedded_sdmmc::{Error, VolumeIdx, VolumeManager};

use crate::{
    drivers::FlushableBlockDevice,
    storage::{
        durable::{DurableArtifact, DurableStorageAdapter},
        repository::{RepositoryCartridgeDigest, RepositoryDocument, RepositoryStreamStorage},
    },
};

use super::{
    AmrnFile, CARTRIDGE_NAME_BYTES, FilesystemManager, KernelTimeSource, TrustStoreArtifact,
    read_trust_store_artifact, stream_repository_file, stream_root_file,
    write_trust_store_artifact,
};

/// Defines the `CARTRIDGE_NAME_BYTES` bound used by this subsystem.
/// Defines the `DELEGATION_NAME_BYTES` bound used by this subsystem.
const DELEGATION_NAME_BYTES: usize = 64 + 5;
/// Defines the `METADATA_NAME_BYTES` bound used by this subsystem.
const METADATA_NAME_BYTES: usize = 16;
/// Defines the `DELEGATIONS_DIRECTORY` bound used by this subsystem.
const DELEGATIONS_DIRECTORY: &str = "delegat";
/// Defines the content-addressed AMRN directory name.
const AMRNS_DIRECTORY: &str = "amrns";
/// Defines the `BUNDLE_MANIFEST_NAME` bound used by this subsystem.
const BUNDLE_MANIFEST_NAME: &str = "bundle.manifest";

/// Names the bounded `RepositoryChunkPet` type used by this subsystem.
type RepositoryChunkPet = fn() -> Result<(), crate::drivers::StorageError>;

/// Performs the `no_repository_chunk_pet` operation for this subsystem.
fn no_repository_chunk_pet() -> Result<(), crate::drivers::StorageError> {
    Ok(())
}

/// Repository metadata encoding selected by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryMetadataFormat {
    /// Legacy canonical JSON repository files.
    JsonV1,
    /// Frozen Binary Metadata v2 repository files.
    BinaryV2,
}

impl RepositoryMetadataFormat {
    /// Returns the filename suffix for the selected metadata encoding.
    const fn extension(self) -> &'static [u8] {
        match self {
            Self::JsonV1 => b".json",
            Self::BinaryV2 => b".dmb",
        }
    }
}

/// FAT32-backed logical repository storage.
///
/// The device is copied into each filesystem pass. The adapter retains no
/// board or controller-specific type.
#[derive(Clone, Copy)]
pub struct FatRepositoryStorage<D> {
    /// Stores the `device` value for this bounded state.
    device: D,
    /// Stores the `metadata_format` value for this bounded state.
    metadata_format: RepositoryMetadataFormat,
    /// Stores the `chunk_pet` value for this bounded state.
    chunk_pet: RepositoryChunkPet,
}

impl<D> FatRepositoryStorage<D> {
    /// Creates a repository adapter over an initialized FAT block device.
    pub const fn new(device: D) -> Self {
        Self::new_with_format(device, RepositoryMetadataFormat::JsonV1)
    }

    /// Creates an adapter with an explicit repository metadata encoding.
    pub const fn new_with_format(device: D, metadata_format: RepositoryMetadataFormat) -> Self {
        Self {
            device,
            metadata_format,
            chunk_pet: no_repository_chunk_pet,
        }
    }

    /// Creates an adapter with an injected bounded chunk-progress hook.
    pub const fn new_with_format_and_pet(
        device: D,
        metadata_format: RepositoryMetadataFormat,
        chunk_pet: RepositoryChunkPet,
    ) -> Self {
        Self {
            device,
            metadata_format,
            chunk_pet,
        }
    }

    /// Performs the `stream_fixed_metadata` operation for this subsystem.
    fn stream_fixed_metadata<F>(
        &self,
        stem: &[u8],
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, embedded_sdmmc::Error<crate::drivers::StorageError>>
    where
        D: Copy + embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
        F: FnMut(&[u8]) -> Result<(), embedded_sdmmc::Error<crate::drivers::StorageError>>,
    {
        let mut name = [0u8; METADATA_NAME_BYTES];
        let name = append_metadata_suffix(stem, self.metadata_format, &mut name)?;
        stream_file(
            self.device,
            "metadata",
            None,
            name,
            chunk,
            consumer,
            self.chunk_pet,
        )
    }
}

/// Opens one content-addressed AMRN cartridge for the existing execution loader.
pub fn with_content_addressed_cartridge<D, F, R, E>(
    device: D,
    digest: RepositoryCartridgeDigest,
    callback: F,
) -> Result<Result<R, E>, Error<crate::drivers::StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
    F: for<'a> FnOnce(AmrnFile<'a, D>) -> Result<R, E>,
{
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Opening verified cartridge for execution"),
    );
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = manager.open_root_dir(volume.to_raw_volume())?;
    let cartridges = match super::open_existing_directory(&manager, root, AMRNS_DIRECTORY) {
        Ok(directory) => directory,
        Err(error) => return super::close_directories(&manager, [root, root, root], 1, Err(error)),
    };
    let mut name = [0u8; CARTRIDGE_NAME_BYTES];
    let name = match append_cartridge_suffix(&digest.0, &mut name) {
        Ok(name) => name,
        Err(error) => {
            return super::close_directories(
                &manager,
                [root, cartridges, cartridges],
                2,
                Err(error),
            );
        }
    };
    let raw_file = match super::read::open_existing_file(&manager, cartridges, name) {
        Ok(file) => file,
        Err(error) => {
            return super::close_directories(
                &manager,
                [root, cartridges, cartridges],
                2,
                Err(error),
            );
        }
    };
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Verified cartridge file opened"),
    );
    let length = match manager.file_length(raw_file) {
        Ok(length) => length,
        Err(error) => {
            return super::close_file_with_error(
                &manager,
                raw_file,
                super::close_directories(&manager, [root, cartridges, cartridges], 2, Err(error)),
            );
        }
    };
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Verified cartridge length read: {}", length),
    );
    let file = match AmrnFile::from_content_addressed(&manager, raw_file, length) {
        Ok(file) => file,
        Err(error) => {
            return super::close_file_with_error(
                &manager,
                raw_file,
                super::close_directories(&manager, [root, cartridges, cartridges], 2, Err(error)),
            );
        }
    };
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Verified cartridge view created"),
    );
    let result = callback(file);
    finish_content_addressed(&manager, [root, cartridges, cartridges], raw_file, result)
}

/// Performs the `finish_content_addressed` operation for this subsystem.
fn finish_content_addressed<D, R, E>(
    manager: &FilesystemManager<D>,
    directories: [embedded_sdmmc::RawDirectory; 3],
    file: embedded_sdmmc::RawFile,
    result: Result<R, E>,
) -> Result<Result<R, E>, Error<crate::drivers::StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>,
{
    let file_result = manager.close_file(file);
    let directory_result = super::close_directories(manager, directories, 2, Ok(()));
    match (result, file_result, directory_result) {
        (Ok(value), Ok(()), Ok(())) => Ok(Ok(value)),
        (Err(error), Ok(()), Ok(())) => Ok(Err(error)),
        (_, Err(error), _) | (_, _, Err(error)) => Err(error),
    }
}

impl<D> DurableStorageAdapter for FatRepositoryStorage<D>
where
    D: Copy
        + embedded_sdmmc::BlockDevice<Error = crate::drivers::StorageError>
        + FlushableBlockDevice<Error = crate::drivers::StorageError>,
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
        self.device.flush().map_err(Error::DeviceError)
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
            RepositoryDocument::Bundle => stream_root_file(
                self.device,
                BUNDLE_MANIFEST_NAME,
                chunk,
                consumer,
                self.chunk_pet,
            ),
            RepositoryDocument::Root => self.stream_fixed_metadata(b"root", chunk, consumer),
            RepositoryDocument::Timestamp => {
                self.stream_fixed_metadata(b"timestamp", chunk, consumer)
            }
            RepositoryDocument::Snapshot => {
                self.stream_fixed_metadata(b"snapshot", chunk, consumer)
            }
            RepositoryDocument::Targets => self.stream_fixed_metadata(b"targets", chunk, consumer),
            RepositoryDocument::Revocations => {
                self.stream_fixed_metadata(b"revocations", chunk, consumer)
            }
            RepositoryDocument::Delegation(identifier) => {
                let mut name = [0u8; DELEGATION_NAME_BYTES];
                let name =
                    append_metadata_suffix(identifier.as_bytes(), self.metadata_format, &mut name)?;
                stream_file(
                    self.device,
                    "metadata",
                    Some(DELEGATIONS_DIRECTORY),
                    name,
                    chunk,
                    consumer,
                    self.chunk_pet,
                )
            }
        }
    }

    fn stream_cartridge<F>(
        &mut self,
        digest: RepositoryCartridgeDigest,
        chunk: &mut [u8],
        consumer: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>,
    {
        let mut name = [0u8; CARTRIDGE_NAME_BYTES];
        let name = append_cartridge_suffix(&digest.0, &mut name)?;
        crate::logging::info(
            crate::logging::BOOT_SUBSYSTEM,
            format_args!(
                "[STORAGE] Streaming content-addressed cartridge; name length={}",
                name.len()
            ),
        );
        let result = stream_file(
            self.device,
            AMRNS_DIRECTORY,
            None,
            name,
            chunk,
            consumer,
            self.chunk_pet,
        );
        if let Err(error) = &result {
            crate::logging::error(
                crate::logging::BOOT_SUBSYSTEM,
                format_args!(
                    "[STORAGE] Content-addressed cartridge stream failed: {:?}",
                    error
                ),
            );
        }
        result
    }
}

/// Performs the `append_metadata_suffix` operation for this subsystem.
fn append_metadata_suffix<'a>(
    stem: &[u8],
    format: RepositoryMetadataFormat,
    output: &'a mut [u8],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    append_suffix(stem, format.extension(), output)
}

/// Performs the `stream_file` operation for this subsystem.
fn stream_file<D, F>(
    device: D,
    first_directory: &str,
    second_directory: Option<&str>,
    file_name: &str,
    chunk: &mut [u8],
    consumer: F,
    chunk_pet: RepositoryChunkPet,
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
        chunk_pet,
    )
}

/// Performs the `append_cartridge_suffix` operation for this subsystem.
fn append_cartridge_suffix<'a>(
    digest: &[u8; 32],
    output: &'a mut [u8; CARTRIDGE_NAME_BYTES],
) -> Result<&'a str, embedded_sdmmc::Error<crate::drivers::StorageError>> {
    let mut length = 0;
    for byte in digest {
        output[length] = hex_digit(byte >> 4);
        output[length + 1] = hex_digit(byte & 0x0F);
        length += 2;
    }
    append_suffix_at(length, b".amrn", output)
}

/// Performs the `append_suffix` operation for this subsystem.
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

/// Performs the `append_suffix_at` operation for this subsystem.
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

/// Converts a nibble into its lowercase hexadecimal digit.
const fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + (value - 10),
    }
}

#[cfg(test)]
mod tests {
    use super::{CARTRIDGE_NAME_BYTES, append_cartridge_suffix};

    #[test]
    fn content_addressed_cartridge_names_use_lowercase_digest_hex() {
        let digest = [0xC6; 32];
        let mut output = [0; CARTRIDGE_NAME_BYTES];
        let mut expected = [0u8; 64];
        for pair in expected.chunks_exact_mut(2) {
            pair.copy_from_slice(b"c6");
        }

        let name = append_cartridge_suffix(&digest, &mut output).unwrap();
        assert_eq!(&name.as_bytes()[..64], &expected);
        assert_eq!(&name.as_bytes()[64..], b".amrn");
    }
}

/// Maps a durable artifact to its trust-store representation.
const fn trust_store_artifact(artifact: DurableArtifact) -> TrustStoreArtifact {
    match artifact {
        DurableArtifact::SlotA => TrustStoreArtifact::Active,
        DurableArtifact::SlotB => TrustStoreArtifact::Candidate,
        DurableArtifact::CommitJournal => TrustStoreArtifact::CommitMarker,
    }
}
