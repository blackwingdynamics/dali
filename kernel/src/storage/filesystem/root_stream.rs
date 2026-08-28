//! Root-level streaming reads for kernel-owned repository artifacts.

use crate::drivers::StorageError;
use embedded_sdmmc::{Error, Mode, VolumeIdx, VolumeManager};

use super::{FilesystemManager, KernelTimeSource, close_root};

/// Streams one root-level artifact through caller-owned bounded storage.
pub fn stream_root_file<D, F>(
    device: D,
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
    let file = match manager.open_long_name_file_in_dir(root, file_name, Mode::ReadOnly) {
        Ok(file) => file,
        Err(error) => return close_root(&manager, root, None, Err(error)),
    };
    let length = match manager.file_length(file) {
        Ok(length) => length,
        Err(error) => return close_root(&manager, root, Some(file), Err(error)),
    };
    let result = stream_file(&manager, file, length, chunk, consumer, chunk_pet);
    match result {
        Ok(value) => close_root(&manager, root, Some(file), Ok(value)),
        Err(error) => {
            let file_result = manager.close_file(file);
            let root_result = manager.close_dir(root);
            match (file_result, root_result) {
                (Ok(()), Ok(())) => Err(error),
                (Err(cleanup_error), _) | (_, Err(cleanup_error)) => Err(cleanup_error),
            }
        }
    }
}

/// Performs the `stream_file` operation for this subsystem.
fn stream_file<D, F>(
    manager: &FilesystemManager<D>,
    file: embedded_sdmmc::RawFile,
    length: u32,
    chunk: &mut [u8],
    mut consumer: F,
    chunk_pet: fn() -> Result<(), StorageError>,
) -> Result<u32, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    F: FnMut(&[u8]) -> Result<(), Error<StorageError>>,
{
    let mut total = 0_u32;
    loop {
        let read = manager.read(file, chunk)?;
        if read == 0 {
            return if total == length {
                Ok(total)
            } else {
                Err(Error::InvalidOffset)
            };
        }
        consumer(&chunk[..read])?;
        chunk_pet().map_err(Error::DeviceError)?;
        total = total
            .checked_add(u32::try_from(read).map_err(|_| Error::InvalidOffset)?)
            .ok_or(Error::InvalidOffset)?;
    }
}
