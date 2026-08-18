//! Bounded enumeration of real root-directory AMRN files.

use core::ops::ControlFlow;

use embedded_sdmmc::{Error, LfnBuffer, Mode, ShortFileName, VolumeIdx, VolumeManager};

use super::{
    AmrnFile, FilesystemManager, KernelTimeSource, LFN_BUFFER_BYTES, MAX_ROOT_AMRN_FILES,
    is_amrn_entry,
};
use crate::drivers::StorageError;

/// Runs a bounded read-only operation for every root AMRN file.
pub fn with_amrn_files<D, F, E>(
    device: D,
    mut callback: F,
) -> Result<Result<(), E>, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    F: for<'a> FnMut(AmrnFile<'a, D>) -> Result<(), E>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let raw_volume = volume.to_raw_volume();
    let root = manager.open_root_dir(raw_volume)?;
    let (names, count) = match collect_amrn_names(&manager, root) {
        Ok(names) => names,
        Err(error) => return super::abort_file_operation(&manager, root, None, error),
    };

    for name in names[..count].iter().flatten().copied() {
        let raw_file = match manager.open_file_in_dir(root, name, Mode::ReadOnly) {
            Ok(raw_file) => raw_file,
            Err(error) => return super::abort_file_operation(&manager, root, None, error),
        };
        let length = match manager.file_length(raw_file) {
            Ok(length) => length,
            Err(error) => {
                return super::abort_file_operation(&manager, root, Some(raw_file), error);
            }
        };
        let result = callback(AmrnFile {
            manager: &manager,
            raw_file,
            length,
            name,
        });
        match finish_multi_file_operation(&manager, raw_file, result) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return match manager.close_dir(root) {
                    Ok(()) => Ok(Err(error)),
                    Err(cleanup_error) => Err(cleanup_error),
                };
            }
            Err(error) => {
                return match manager.close_dir(root) {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(cleanup_error),
                };
            }
        }
    }
    manager.close_dir(root)?;
    Ok(Ok(()))
}

fn finish_multi_file_operation<D, E>(
    manager: &FilesystemManager<D>,
    raw_file: embedded_sdmmc::RawFile,
    result: Result<(), E>,
) -> Result<Result<(), E>, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let file_result = manager.close_file(raw_file);
    match result {
        Err(error) => match file_result {
            Ok(()) => Ok(Err(error)),
            Err(cleanup_error) => Err(cleanup_error),
        },
        Ok(()) => {
            file_result?;
            Ok(Ok(()))
        }
    }
}

fn collect_amrn_names<D>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
) -> Result<([Option<ShortFileName>; MAX_ROOT_AMRN_FILES], usize), Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut lfn_storage = [0u8; LFN_BUFFER_BYTES];
    let mut lfn_buffer = LfnBuffer::new(&mut lfn_storage);
    let mut names = [None; MAX_ROOT_AMRN_FILES];
    let mut count = 0;
    let mut overflow = false;
    manager.iterate_dir_lfn(root, &mut lfn_buffer, |entry, long_name| {
        if is_amrn_entry(entry, long_name) {
            if let Some(name) = names.get_mut(count) {
                *name = Some(entry.name);
                count += 1;
            } else {
                overflow = true;
            }
        }
        ControlFlow::Continue(())
    })?;
    if overflow {
        return Err(Error::Unsupported);
    }
    Ok((names, count))
}
