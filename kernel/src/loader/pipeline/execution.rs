//! Streaming ABI v3 package loading for the feature-gated kernel path.

use dali_amrn::{Crc32, v2};

use crate::security::launch::{self, LaunchFrame};
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    runtime::{application::lifecycle::ApplicationLifecycle, memory::slots::SlotAllocation},
    storage::filesystem::AmrnFile,
};

#[cfg(feature = "abi-context-switch")]
use crate::runtime::scheduling::{record::ScheduledContext, saved_state::UNPRIVILEGED_PSP_CONTROL};

/// Values retained after an ABI v3 package has been copied into SRAM.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadedApplication {
    /// Validated entry address before the Thumb bit is applied.
    pub entry_address: u32,
    /// Initial PSP value at the top of the declared stack reservation.
    pub psp_top: u32,
    /// Kernel-generated frame for the future unprivileged exception return.
    pub(crate) launch_frame: LaunchFrame,
    /// Manifest slot selected for the loaded application's code and data.
    pub(crate) slot: dali_targets::IsolationSlot,
    /// Reserved allocation retained for runtime ownership.
    pub(crate) allocation: SlotAllocation,
    /// Identity lifecycle for v4 packages; legacy packages remain untracked.
    pub(crate) lifecycle: Option<ApplicationLifecycle>,
}

impl LoadedApplication {
    /// Builds the initial scheduler record from the validated launch frame.
    #[cfg(feature = "abi-context-switch")]
    pub(crate) const fn scheduler_context(self) -> ScheduledContext {
        ScheduledContext::initial(
            self.launch_frame.psp,
            UNPRIVILEGED_PSP_CONTROL,
            self.launch_frame.exception_return,
            self.slot,
        )
    }
}

/// Fixed-capacity set of packages loaded into manifest-owned slots.
pub(crate) struct LoadedApplications<const CAPACITY: usize> {
    entries: [Option<LoadedApplication>; CAPACITY],
    length: usize,
}

impl<const CAPACITY: usize> LoadedApplications<CAPACITY> {
    /// Creates an empty bounded application set.
    pub(crate) const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            length: 0,
        }
    }

    /// Adds one loaded package without allocating.
    pub(crate) fn push(&mut self, application: LoadedApplication) -> bool {
        let Some(entry) = self.entries.get_mut(self.length) else {
            return false;
        };
        *entry = Some(application);
        self.length += 1;
        true
    }

    /// Returns the first package selected for the current single-context runtime.
    pub(crate) fn first(&self) -> Option<LoadedApplication> {
        self.entries.first().copied().flatten()
    }

    /// Iterates over loaded packages in deterministic slot-selection order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &LoadedApplication> {
        self.entries[..self.length]
            .iter()
            .filter_map(Option::as_ref)
    }

    /// Returns the number of packages copied into manifest-owned slots.
    pub(crate) const fn len(&self) -> usize {
        self.length
    }

    /// Wraps one loaded package in a bounded set.
    pub(crate) fn single(application: LoadedApplication) -> Self {
        let mut applications = Self::new();
        let _ = applications.push(application);
        applications
    }
}

/// Reads, validates, and copies one ABI v3 package using bounded storage reads.
pub(crate) fn load_file<D>(
    file: AmrnFile<'_, D>,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let header = read_header(&file)?;
    let (contract, allocation) = target_contract(&header, slot_manager)?;
    let header =
        v2::parse_header(&header, contract).map_err(super::LoaderError::CurrentAbiPackage)?;
    let expected_length = package_length(header).ok_or(super::LoaderError::CurrentAbiPackage(
        v2::Error::InvalidPayload,
    ))?;
    if u64::from(file.length()) != u64::from(expected_length) {
        return Err(super::LoaderError::CurrentAbiPackage(
            v2::Error::InvalidPayload,
        ));
    }
    validate_payload(&file, header)?;
    copy_segments(&file, header)?;
    let entry_address = header
        .code_load_address
        .checked_add(header.execution_offset)
        .ok_or(super::LoaderError::CurrentAbiPackage(
            v2::Error::AddressOverflow,
        ))?;
    let stack_origin = header
        .data_load_address
        .checked_add(header.data_init_size)
        .and_then(|address| address.checked_add(header.data_zero_size))
        .ok_or(super::LoaderError::CurrentAbiPackage(
            v2::Error::AddressOverflow,
        ))?;
    let psp_top = stack_origin.checked_add(header.stack_size).ok_or(
        super::LoaderError::CurrentAbiPackage(v2::Error::AddressOverflow),
    )?;
    let launch_frame = launch::prepare(entry_address, stack_origin, header.stack_size).map_err(
        |error| match error {
            launch::LaunchError::InvalidEntry => {
                super::LoaderError::CurrentAbiPackage(v2::Error::InvalidExecutionOffset)
            }
            launch::LaunchError::InvalidStack => {
                super::LoaderError::CurrentAbiPackage(v2::Error::RegionOverflow)
            }
        },
    )?;
    launch::materialize(launch_frame);
    Ok(LoadedApplication {
        entry_address,
        psp_top,
        launch_frame,
        slot: allocation.slot(),
        allocation,
        lifecycle: None,
    })
}

fn target_contract(
    bytes: &[u8; v2::HEADER_SIZE],
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<(v2::Contract, SlotAllocation), super::LoaderError> {
    let target = crate::platform::TARGET_PROFILE;
    let isolation = target
        .memory
        .isolation
        .ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
    let mut last_error = v2::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = v2::Contract {
            target_id: target.amrn_target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match v2::parse_header(bytes, contract) {
            Ok(_) => {
                let allocation = slot_manager
                    .reserve(slot)
                    .map_err(super::LoaderError::SlotManager)?;
                return Ok((contract, allocation));
            }
            Err(error) => last_error = error,
        }
    }
    Err(super::LoaderError::CurrentAbiPackage(last_error))
}

fn read_header<D>(file: &AmrnFile<'_, D>) -> Result<[u8; v2::HEADER_SIZE], super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header = [0; v2::HEADER_SIZE];
    read_exact(file, &mut header).map_err(super::LoaderError::Filesystem)?;
    Ok(header)
}

fn package_length(header: v2::Header) -> Option<u32> {
    u32::try_from(v2::HEADER_SIZE)
        .ok()?
        .checked_add(header.code_size)?
        .checked_add(header.data_init_size)
}

fn validate_payload<D>(file: &AmrnFile<'_, D>, header: v2::Header) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let _header = read_header(file)?;
    let payload_size = header.code_size.checked_add(header.data_init_size).ok_or(
        super::LoaderError::CurrentAbiPackage(v2::Error::InvalidPayload),
    )?;
    let mut checksum = Crc32::new();
    let mut remaining = usize::try_from(payload_size)
        .map_err(|_| super::LoaderError::CurrentAbiPackage(v2::Error::InvalidPayload))?;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        read_exact(file, &mut chunk[..chunk_size]).map_err(super::LoaderError::Filesystem)?;
        checksum.update(&chunk[..chunk_size]);
        remaining -= chunk_size;
    }
    if checksum.finish() != header.crc32 {
        return Err(super::LoaderError::CurrentAbiPackage(
            v2::Error::CrcMismatch,
        ));
    }
    Ok(())
}

fn copy_segments<D>(file: &AmrnFile<'_, D>, header: v2::Header) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let _header = read_header(file)?;
    copy_segment(file, header.code_load_address, header.code_size)?;
    copy_segment(file, header.data_load_address, header.data_init_size)?;
    let zero_start = header
        .data_load_address
        .checked_add(header.data_init_size)
        .ok_or(super::LoaderError::CurrentAbiPackage(
            v2::Error::AddressOverflow,
        ))?;
    zero_segment(zero_start, header.data_zero_size)?;
    Ok(())
}

fn copy_segment<D>(
    file: &AmrnFile<'_, D>,
    destination: u32,
    size: u32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut remaining = usize::try_from(size)
        .map_err(|_| super::LoaderError::CurrentAbiPackage(v2::Error::InvalidPayload))?;
    let mut offset = 0usize;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        read_exact(file, &mut chunk[..chunk_size]).map_err(super::LoaderError::Filesystem)?;
        let address = usize::try_from(destination)
            .ok()
            .and_then(|value| value.checked_add(offset))
            .ok_or(super::LoaderError::CurrentAbiPackage(
                v2::Error::AddressOverflow,
            ))?;
        let target = unsafe {
            // SAFETY: v2 header validation proved the complete segment fits its
            // target SRAM region before this second, copy-only pass begins.
            core::slice::from_raw_parts_mut(address as *mut u8, chunk_size)
        };
        target.copy_from_slice(&chunk[..chunk_size]);
        offset += chunk_size;
        remaining -= chunk_size;
    }
    Ok(())
}

fn zero_segment(destination: u32, size: u32) -> Result<(), super::LoaderError> {
    let length = usize::try_from(size)
        .map_err(|_| super::LoaderError::CurrentAbiPackage(v2::Error::InvalidPayload))?;
    if length == 0 {
        return Ok(());
    }
    let target = unsafe {
        // SAFETY: v2 header validation proved the zero-init range is inside the
        // target data region and does not overlap the reserved PSP stack.
        core::slice::from_raw_parts_mut(destination as *mut u8, length)
    };
    target.fill(0);
    Ok(())
}

fn read_exact<D>(
    file: &AmrnFile<'_, D>,
    buffer: &mut [u8],
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut offset = 0;
    while offset < buffer.len() {
        let read = file.read(&mut buffer[offset..])?;
        if read == 0 {
            return Err(embedded_sdmmc::Error::EndOfFile);
        }
        offset += read;
    }
    Ok(())
}
