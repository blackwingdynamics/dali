//! Explicit FAT write operations for durable kernel-owned artifacts.

use crate::drivers::StorageError;
use embedded_sdmmc::{Error, Mode, RawFile, VolumeIdx, VolumeManager};

use super::{FilesystemManager, KernelTimeSource};

/// Writes one bounded root-level artifact and flushes its directory entry.
///
/// The caller owns the artifact contract and must provide a bounded short-file
/// name. This operation does not provide a multi-file transaction; trust-store
/// activation must sequence candidate and commit-marker files above it.
pub fn write_root_file<D>(device: D, name: &str, contents: &[u8]) -> Result<(), Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = manager.open_root_dir(volume.to_raw_volume())?;
    let file = match manager.open_file_in_dir(root, name, Mode::ReadWriteCreateOrTruncate) {
        Ok(file) => file,
        Err(error) => return close_root(&manager, root, None, Err(error)),
    };
    let result = manager.write(file, contents);
    close_root(&manager, root, Some(file), result)
}

fn close_root<D>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
    file: Option<RawFile>,
    result: Result<(), Error<StorageError>>,
) -> Result<(), Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let file_result = file.map_or(Ok(()), |file| manager.close_file(file));
    let directory_result = manager.close_dir(root);
    match result {
        Err(error) => match (file_result, directory_result) {
            (Ok(()), Ok(())) => Err(error),
            (Err(cleanup_error), _) | (_, Err(cleanup_error)) => Err(cleanup_error),
        },
        Ok(()) => {
            file_result?;
            directory_result?;
            Ok(())
        }
    }
}
