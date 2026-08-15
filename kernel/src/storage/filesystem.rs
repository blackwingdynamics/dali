//! Read-only FAT filesystem operations owned by the storage subsystem.

use core::ops::ControlFlow;

use crate::storage::StorageError;
use embedded_sdmmc::{Error, TimeSource, Timestamp, VolumeIdx, VolumeManager};

/// The first package base name used by the MVP acceptance application.
pub const MVP_PACKAGE_BASE_NAME: &[u8] = b"HELLO";

/// The package extension recognized by the MVP root-directory scan.
pub const AMRN_EXTENSION: &[u8] = b"AMRN";

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
    /// Whether the named MVP package was found.
    pub mvp_package_found: bool,
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
    let mut report = RootDirectoryReport {
        amrn_file_count: 0,
        mvp_package_found: false,
    };
    root.iterate_dir(|entry| {
        if entry.name.extension() == AMRN_EXTENSION {
            report.amrn_file_count = report.amrn_file_count.saturating_add(1);
            report.mvp_package_found |= entry.name.base_name() == MVP_PACKAGE_BASE_NAME;
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
