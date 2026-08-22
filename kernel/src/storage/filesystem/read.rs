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

/// Reads a repository file from a bounded FAT directory path.
pub fn read_repository_file<D>(
    device: D,
    first_directory: &str,
    second_directory: Option<&str>,
    file_name: &str,
    output: &mut [u8],
) -> Result<usize, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = manager.open_root_dir(volume.to_raw_volume())?;
    let first = match manager.open_dir(root, first_directory) {
        Ok(directory) => directory,
        Err(error) => return close_directories(&manager, [root, root, root], 1, Err(error)),
    };
    let (directory, directory_count) = match second_directory {
        Some(name) => match manager.open_dir(first, name) {
            Ok(directory) => (directory, 3),
            Err(error) => return close_directories(&manager, [root, first, first], 2, Err(error)),
        },
        None => (first, 2),
    };
    let result = read_named_file(&manager, directory, file_name, output);
    close_directories(&manager, [root, first, directory], directory_count, result)
}

/// Streams a repository file through a caller-owned bounded chunk.
pub fn stream_repository_file<D, F>(
    device: D,
    first_directory: &str,
    second_directory: Option<&str>,
    file_name: &str,
    chunk: &mut [u8],
    consumer: F,
    chunk_pet: fn() -> Result<(), StorageError>,
) -> Result<u32, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    F: FnMut(&[u8]) -> Result<(), Error<StorageError>>,
{
    if chunk.is_empty() {
        return Err(Error::InvalidOffset);
    }
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = manager.open_root_dir(volume.to_raw_volume())?;
    let first = match manager.open_dir(root, first_directory) {
        Ok(directory) => directory,
        Err(error) => return close_directories(&manager, [root, root, root], 1, Err(error)),
    };
    let (directory, directory_count) = match second_directory {
        Some(name) => match manager.open_dir(first, name) {
            Ok(directory) => (directory, 3),
            Err(error) => return close_directories(&manager, [root, first, first], 2, Err(error)),
        },
        None => (first, 2),
    };
    let result = stream_named_file(&manager, directory, file_name, chunk, consumer, chunk_pet);
    close_directories(&manager, [root, first, directory], directory_count, result)
}

fn read_named_file<D>(
    manager: &FilesystemManager<D>,
    directory: embedded_sdmmc::RawDirectory,
    file_name: &str,
    output: &mut [u8],
) -> Result<usize, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let file = manager.open_long_name_file_in_dir(directory, file_name, Mode::ReadOnly)?;
    let length = match manager.file_length(file) {
        Ok(length) => length,
        Err(error) => return close_file_with_error(manager, file, Err(error)),
    };
    let length = match usize::try_from(length) {
        Ok(length) => length,
        Err(_) => return close_file_with_error(manager, file, Err(Error::InvalidOffset)),
    };
    if length > output.len() {
        return close_file_with_error(manager, file, Err(Error::InvalidOffset));
    }
    let result = read_exact(manager, file, &mut output[..length]).map(|()| length);
    close_file_with_error(manager, file, result)
}

fn stream_named_file<D, F>(
    manager: &FilesystemManager<D>,
    directory: embedded_sdmmc::RawDirectory,
    file_name: &str,
    chunk: &mut [u8],
    mut consumer: F,
    chunk_pet: fn() -> Result<(), StorageError>,
) -> Result<u32, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    F: FnMut(&[u8]) -> Result<(), Error<StorageError>>,
{
    let file = manager.open_long_name_file_in_dir(directory, file_name, Mode::ReadOnly)?;
    let length = match manager.file_length(file) {
        Ok(length) => length,
        Err(error) => return close_file_with_error(manager, file, Err(error)),
    };
    let result = (|| {
        let mut total = 0_u32;
        loop {
            let read = manager.read(file, chunk)?;
            if read == 0 {
                if total != length {
                    return Err(Error::InvalidOffset);
                }
                return Ok(total);
            }
            consumer(&chunk[..read])?;
            chunk_pet().map_err(Error::DeviceError)?;
            total = total
                .checked_add(u32::try_from(read).map_err(|_| Error::InvalidOffset)?)
                .ok_or(Error::InvalidOffset)?;
        }
    })();
    close_file_with_error(manager, file, result)
}

pub(crate) fn close_file_with_error<D, R>(
    manager: &FilesystemManager<D>,
    file: RawFile,
    result: Result<R, Error<StorageError>>,
) -> Result<R, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let close_result = manager.close_file(file);
    match result {
        Ok(value) => {
            close_result?;
            Ok(value)
        }
        Err(error) => match close_result {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(cleanup_error),
        },
    }
}

pub(crate) fn close_directories<D, R>(
    manager: &FilesystemManager<D>,
    directories: [embedded_sdmmc::RawDirectory; 3],
    count: usize,
    result: Result<R, Error<StorageError>>,
) -> Result<R, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut cleanup_error = None;
    for directory in directories[..count].iter().rev() {
        if let Err(error) = manager.close_dir(*directory) {
            cleanup_error = Some(error);
        }
    }
    match result {
        Ok(value) => cleanup_error.map_or(Ok(value), Err),
        Err(error) => Err(cleanup_error.unwrap_or(error)),
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

pub(crate) fn close_root<D, R>(
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
