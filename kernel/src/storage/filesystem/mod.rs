//! Bounded FAT filesystem operations owned by the storage subsystem.

use core::ops::ControlFlow;

use crate::drivers::StorageError;
use embedded_sdmmc::{
    DirEntry, Error, LfnBuffer, Mode, RawFile, ShortFileName, TimeSource, Timestamp, VolumeIdx,
    VolumeManager,
};

mod artifacts;
mod multi;
mod read;
mod repository;
mod root_stream;
#[cfg(test)]
mod tests;
mod write;

pub use artifacts::{
    TRUST_STORE_ACTIVE_FILE, TRUST_STORE_CANDIDATE_FILE, TRUST_STORE_COMMIT_FILE,
    TrustStoreArtifact, read_trust_store_artifact, write_trust_store_artifact,
};
pub use multi::with_amrn_files;
pub(crate) use read::open_existing_directory;
pub(crate) use read::{close_directories, close_file_with_error, close_root};
pub use read::{read_repository_file, read_root_file, stream_repository_file};
pub use repository::{
    FatRepositoryStorage, RepositoryMetadataFormat, with_content_addressed_cartridge,
};
pub(crate) use root_stream::stream_root_file;
pub use write::write_root_file;

/// The cartridge extension recognized by the MVP root-directory scan.
pub const AMRN_EXTENSION: &[u8] = b"AMRN";
/// Maximum number of root AMRN files processed in one bounded scan.
pub const MAX_ROOT_AMRN_FILES: usize = 4;
// These bounds are defined by the FAT long-file-name representation and the
// UTF-8 encoding used by `embedded-sdmmc`.
/// Defines the `FAT_LFN_MAX_CHARACTERS` bound used by this subsystem.
const FAT_LFN_MAX_CHARACTERS: usize = 255;
/// Defines the `UTF8_MAX_BYTES_PER_CHARACTER` bound used by this subsystem.
const UTF8_MAX_BYTES_PER_CHARACTER: usize = 3;
/// Defines the maximum repository cartridge filename length.
pub(crate) const CARTRIDGE_NAME_BYTES: usize = 64 + 5;
/// Defines the `LFN_BUFFER_BYTES` bound used by this subsystem.
const LFN_BUFFER_BYTES: usize = FAT_LFN_MAX_CHARACTERS * UTF8_MAX_BYTES_PER_CHARACTER;
/// Storage reserved for repository LFN scans outside the constrained boot stack.
#[unsafe(link_section = ".repository_workspace")]
static mut REPOSITORY_LFN_STORAGE: [u8; CARTRIDGE_NAME_BYTES * UTF8_MAX_BYTES_PER_CHARACTER] =
    [0; CARTRIDGE_NAME_BYTES * UTF8_MAX_BYTES_PER_CHARACTER];

/// Provides the single-threaded repository LFN workspace.
pub(crate) fn with_repository_lfn_buffer<R>(operation: impl FnOnce(&mut LfnBuffer<'_>) -> R) -> R {
    // SAFETY: repository loading is single-threaded and completes before
    // application contexts or scheduler interrupts can access this workspace.
    unsafe {
        let storage = core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(REPOSITORY_LFN_STORAGE).cast::<u8>(),
            CARTRIDGE_NAME_BYTES * UTF8_MAX_BYTES_PER_CHARACTER,
        );
        let mut buffer = LfnBuffer::new(storage);
        operation(&mut buffer)
    }
}

/// Defines the `DEFAULT_TIMESTAMP` bound used by this subsystem.
const DEFAULT_TIMESTAMP: Timestamp = Timestamp {
    year_since_1970: 56,
    zero_indexed_month: 0,
    zero_indexed_day: 0,
    hours: 0,
    minutes: 0,
    seconds: 0,
};

/// Summary of the read-only root-directory scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootDirectoryReport {
    /// Number of regular root entries with the AMRN extension.
    pub amrn_file_count: u32,
}

/// Bounded selection result for the current root-cartridge contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootCartridgeSelection {
    /// No root AMRN cartridge is available.
    None,
    /// Exactly one root AMRN cartridge is eligible for selection.
    Single,
    /// More than one root AMRN cartridge requires an identity-aware policy.
    Ambiguous,
}

impl RootDirectoryReport {
    /// Classifies the root directory without inferring identity from filenames.
    pub const fn cartridge_selection(self) -> RootCartridgeSelection {
        match self.amrn_file_count {
            0 => RootCartridgeSelection::None,
            1 => RootCartridgeSelection::Single,
            _ => RootCartridgeSelection::Ambiguous,
        }
    }
}

/// Defines the `MAX_OPEN_DIRECTORIES` bound used by this subsystem.
const MAX_OPEN_DIRECTORIES: usize = 4;
/// Defines the `MAX_OPEN_FILES` bound used by this subsystem.
const MAX_OPEN_FILES: usize = 4;
/// Defines the `MAX_OPEN_VOLUMES` bound used by this subsystem.
const MAX_OPEN_VOLUMES: usize = 1;

/// Names the bounded `FilesystemManager` type used by this subsystem.
type FilesystemManager<D> =
    VolumeManager<D, KernelTimeSource, MAX_OPEN_DIRECTORIES, MAX_OPEN_FILES, MAX_OPEN_VOLUMES>;

/// A read-only stream for the single root AMRN cartridge selected by the loader.
pub struct AmrnFile<'a, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    /// Stores the `manager` value for this bounded state.
    manager: &'a FilesystemManager<D>,
    /// Stores the `raw_file` value for this bounded state.
    raw_file: RawFile,
    /// Stores the `length` value for this bounded state.
    length: u32,
    /// Stores the `name` value for this bounded state.
    name: ShortFileName,
}

impl<D> AmrnFile<'_, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    /// Returns the file length recorded in the FAT directory entry.
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// Returns the short directory key used to reopen this file.
    pub const fn name(&self) -> ShortFileName {
        self.name
    }

    /// Reads the next bounded portion of the cartridge.
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Error<StorageError>> {
        self.manager.read(self.raw_file, buffer)
    }

    /// Rewinds the stream to the beginning of the cartridge.
    pub fn rewind(&self) -> Result<(), Error<StorageError>> {
        self.manager.file_seek_from_start(self.raw_file, 0)
    }
}

impl<'a, D> AmrnFile<'a, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    /// Constructs or updates `from_content_addressed` for this subsystem.
    pub(crate) fn from_content_addressed(
        manager: &'a FilesystemManager<D>,
        raw_file: RawFile,
        length: u32,
    ) -> Result<Self, Error<StorageError>> {
        let name = ShortFileName::create_from_str("PKG.AMR").map_err(|_| Error::InvalidOffset)?;
        Ok(Self {
            manager,
            raw_file,
            length,
            name,
        })
    }
}

/// Supplies a deterministic timestamp because the MVP has no RTC integration.
pub struct KernelTimeSource;

impl TimeSource for KernelTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        DEFAULT_TIMESTAMP
    }
}

/// Scans the first FAT volume root directory for AMRN cartridges.
pub fn scan_root_directory<D>(device: D) -> Result<RootDirectoryReport, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let root = volume.open_root_dir()?;
    let mut lfn_storage = [0u8; LFN_BUFFER_BYTES];
    let mut lfn_buffer = LfnBuffer::new(&mut lfn_storage);
    let mut report = RootDirectoryReport { amrn_file_count: 0 };
    root.iterate_dir_lfn(&mut lfn_buffer, |entry, long_name| {
        if is_amrn_entry(entry, long_name) {
            report.amrn_file_count = report.amrn_file_count.saturating_add(1);
        }
        ControlFlow::Continue(())
    })?;
    drop(root);
    // `embedded-sdmmc` updates the FAT32 FSInfo sector while closing a volume.
    // The kernel deliberately exposes a read-only block device, so preserve
    // the open raw handle and let the manager finish without a write-back.
    let _raw_volume = volume.to_raw_volume();
    Ok(report)
}

/// Runs a bounded read-only operation on the single root AMRN cartridge.
pub fn with_amrn_file<D, F, R, E>(
    device: D,
    callback: F,
) -> Result<Result<R, E>, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    F: for<'a> FnOnce(AmrnFile<'a, D>) -> Result<R, E>,
{
    let manager = VolumeManager::new(device, KernelTimeSource);
    let volume = manager.open_volume(VolumeIdx(0))?;
    let raw_volume = volume.to_raw_volume();
    let root = manager.open_root_dir(raw_volume)?;
    let candidate = match find_single_amrn(&manager, root) {
        Ok(candidate) => candidate,
        Err(error) => return abort_file_operation(&manager, root, None, error),
    };
    let raw_file = match manager.open_file_in_dir(root, candidate.name, Mode::ReadOnly) {
        Ok(raw_file) => raw_file,
        Err(error) => return abort_file_operation(&manager, root, None, error),
    };
    let length = match manager.file_length(raw_file) {
        Ok(length) => length,
        Err(error) => return abort_file_operation(&manager, root, Some(raw_file), error),
    };
    let result = callback(AmrnFile {
        manager: &manager,
        raw_file,
        length,
        name: candidate.name,
    });
    finish_file_operation(&manager, root, raw_file, result)
}

/// Runs a bounded read-only operation for every root AMRN file.
fn abort_file_operation<D, R, E>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
    raw_file: Option<RawFile>,
    error: Error<StorageError>,
) -> Result<Result<R, E>, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let file_result = match raw_file {
        Some(raw_file) => manager.close_file(raw_file),
        None => Ok(()),
    };
    let dir_result = manager.close_dir(root);
    match (file_result, dir_result) {
        (Ok(()), Ok(())) => Err(error),
        (Err(cleanup_error), _) | (_, Err(cleanup_error)) => Err(cleanup_error),
    }
}

#[derive(Clone, Copy)]
/// Represents the private `CartridgeCandidate` state for this subsystem.
struct CartridgeCandidate {
    /// Stores the discovered short filename.
    name: ShortFileName,
}

/// Performs the `find_single_amrn` operation for this subsystem.
fn find_single_amrn<D>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
) -> Result<CartridgeCandidate, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut lfn_storage = [0u8; LFN_BUFFER_BYTES];
    let mut lfn_buffer = LfnBuffer::new(&mut lfn_storage);
    let mut candidate = None;
    let mut count = 0u8;
    manager.iterate_dir_lfn(root, &mut lfn_buffer, |entry, long_name| {
        if is_amrn_entry(entry, long_name) {
            count = count.saturating_add(1);
            if candidate.is_none() {
                candidate = Some(CartridgeCandidate { name: entry.name });
            }
        }
        ControlFlow::Continue(())
    })?;
    match (
        RootDirectoryReport {
            amrn_file_count: u32::from(count),
        }
        .cartridge_selection(),
        candidate,
    ) {
        (RootCartridgeSelection::None, _) => Err(Error::NotFound),
        (RootCartridgeSelection::Single, Some(candidate)) => Ok(candidate),
        (RootCartridgeSelection::Ambiguous, _) => Err(Error::Unsupported),
        (RootCartridgeSelection::Single, None) => Err(Error::NotFound),
    }
}

/// Performs the `finish_file_operation` operation for this subsystem.
fn finish_file_operation<D, R, E>(
    manager: &FilesystemManager<D>,
    root: embedded_sdmmc::RawDirectory,
    raw_file: RawFile,
    result: Result<R, E>,
) -> Result<Result<R, E>, Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
    E: Sized,
{
    let file_result = manager.close_file(raw_file);
    let dir_result = manager.close_dir(root);
    match result {
        Err(error) => match (file_result, dir_result) {
            (Ok(()), Ok(())) => Ok(Err(error)),
            (Err(cleanup_error), _) | (_, Err(cleanup_error)) => Err(cleanup_error),
        },
        Ok(value) => {
            file_result?;
            dir_result?;
            Ok(Ok(value))
        }
    }
}

/// Performs the `is_amrn_entry` operation for this subsystem.
fn is_amrn_entry(entry: &DirEntry, long_name: Option<&str>) -> bool {
    if entry.name.extension().eq_ignore_ascii_case(AMRN_EXTENSION) {
        return true;
    }
    let Some(long_name) = long_name else {
        return false;
    };
    let Some((_, extension)) = long_name.rsplit_once('.') else {
        return false;
    };
    extension.as_bytes().eq_ignore_ascii_case(AMRN_EXTENSION)
}
