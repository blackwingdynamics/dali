//! Read-only FAT filesystem operations owned by the storage subsystem.

use core::ops::ControlFlow;

use crate::storage::StorageError;
use embedded_sdmmc::{DirEntry, Error, LfnBuffer, TimeSource, Timestamp, VolumeIdx, VolumeManager};

/// The package extension recognized by the MVP root-directory scan.
pub const AMRN_EXTENSION: &[u8] = b"AMRN";
// These bounds are defined by the FAT long-file-name representation and the
// UTF-8 encoding used by `embedded-sdmmc`.
const FAT_LFN_MAX_CHARACTERS: usize = 255;
const UTF8_MAX_BYTES_PER_CHARACTER: usize = 3;
const LFN_BUFFER_BYTES: usize = FAT_LFN_MAX_CHARACTERS * UTF8_MAX_BYTES_PER_CHARACTER;

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

/// Supplies a deterministic timestamp because the MVP has no RTC integration.
pub struct KernelTimeSource;

impl TimeSource for KernelTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        DEFAULT_TIMESTAMP
    }
}

/// Scans the first FAT volume root directory for AMRN packages.
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
