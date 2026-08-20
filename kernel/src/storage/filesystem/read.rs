//! Bounded FAT read operations for kernel-owned artifacts.

use crate::drivers::StorageError;
use embedded_sdmmc::{Error, Mode, RawFile, VolumeIdx, VolumeManager};

use super::{FilesystemManager, KernelTimeSource};

/// Reads one root-level artifact into caller-owned bounded storage.
pub fn read_root_file<D>(
    device: D,
    name: &str,
    output: &mut [u8],
) -> Result<usize, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = manager.open_root_dir(volume.to_raw_volume())?;
    let file = match manager.open_file_in_dir(root, name, Mode::ReadOnly) {
        Ok(file) => file,
        Err(error) => return close_root(&manager, root, None, Err(error)),
    };
    let length = match manager.file_length(file) {
        Ok(length) => length,
        Err(error) => return close_root(&manager, root, Some(file), Err(error)),
    };
    let length = match usize::try_from(length) {
        Ok(length) => length,
        Err(_) => return close_root(&manager, root, Some(file), Err(Error::InvalidOffset)),
    };
    if length > output.len() {
        return close_root(&manager, root, Some(file), Err(Error::InvalidOffset));
    }
    let result = read_exact(&manager, file, &mut output[..length]);
    match result {
        Ok(()) => close_root(&manager, root, Some(file), Ok(length)),
        Err(error) => close_root(&manager, root, Some(file), Err(error)),
    }
}

fn read_exact<D>(
    manager: &FilesystemManager<D>,
    file: RawFile,
    output: &mut [u8],
) -> Result<(), Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut offset = 0;
    while offset < output.len() {
        let read = manager.read(file, &mut output[offset..])?;
        if read == 0 {
            return Err(Error::InvalidOffset);
        }
        offset += read;
    }
    Ok(())
}

fn close_root<D, R>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
    file: Option<RawFile>,
    result: Result<R, Error<StorageError>>,
) -> Result<R, Error<StorageError>>
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
        Ok(value) => {
            file_result?;
            directory_result?;
            Ok(value)
        }
    }
}
