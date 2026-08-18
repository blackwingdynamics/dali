//! Bounded streaming loader for AMRN format 3 relocation packages.

use dali_amrn::{Crc32, v3};

use crate::security::launch::{self, LaunchFrame};
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    storage::filesystem::AmrnFile,
};

use super::execution::LoadedApplication;

const RELOCATION_BYTES: usize = v3::RELOCATION_ENTRY_SIZE;

pub(crate) fn load_file<D>(
    file: AmrnFile<'_, D>,
    slot_manager: &mut crate::runtime::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let header = read_header(&file)?;
    let (header, contract, slot) = parse_target_header(&header, slot_manager)?;
    let relocation_offset = u32::try_from(v3::HEADER_SIZE)
        .ok()
        .and_then(|offset| offset.checked_add(header.code_size))
        .and_then(|offset| offset.checked_add(header.data_init_size))
        .ok_or(super::LoaderError::V3RelocationPackage(
            v3::Error::InvalidPayload,
        ))?;
    if header.relocation_offset != relocation_offset {
        return Err(super::LoaderError::V3RelocationPackage(
            v3::Error::InvalidPayload,
        ));
    }
    let expected_length = package_length(header).ok_or(super::LoaderError::V3RelocationPackage(
        v3::Error::InvalidPayload,
    ))?;
    if u64::from(file.length()) != u64::from(expected_length) {
        return Err(super::LoaderError::V3RelocationPackage(
            v3::Error::InvalidPayload,
        ));
    }
    validate_payload(&file, header, contract)?;
    copy_segments_and_relocate(&file, header, contract)?;
    let entry_address = header
        .code_load_address
        .checked_add(header.execution_offset)
        .ok_or(super::LoaderError::V3RelocationPackage(
            v3::Error::AddressOverflow,
        ))?;
    let stack_origin = header
        .data_load_address
        .checked_add(header.data_init_size)
        .and_then(|address| address.checked_add(header.data_zero_size))
        .ok_or(super::LoaderError::V3RelocationPackage(
            v3::Error::AddressOverflow,
        ))?;
    let launch_frame = prepare_launch(entry_address, stack_origin, header.stack_size)?;
    launch::materialize(launch_frame);
    Ok(LoadedApplication {
        entry_address,
        psp_top: stack_origin.checked_add(header.stack_size).ok_or(
            super::LoaderError::V3RelocationPackage(v3::Error::AddressOverflow),
        )?,
        launch_frame,
        slot,
    })
}

fn parse_target_header(
    bytes: &[u8; v3::HEADER_SIZE],
    slot_manager: &mut crate::runtime::slots::SlotManager,
) -> Result<(v3::Header, v3::Contract, dali_targets::IsolationSlot), super::LoaderError> {
    let target = crate::platform::TARGET_PROFILE;
    let isolation = target
        .memory
        .isolation
        .ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
    let mut last_error = v3::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = v3::Contract {
            target_id: target.amrn_target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match v3::parse_header(bytes, contract) {
            Ok(header) => {
                let allocation = slot_manager
                    .reserve(slot)
                    .map_err(super::LoaderError::SlotManager)?;
                return Ok((header, contract, allocation.slot()));
            }
            Err(error) => last_error = error,
        }
    }
    Err(super::LoaderError::V3RelocationPackage(last_error))
}

fn prepare_launch(
    entry_address: u32,
    stack_origin: u32,
    stack_size: u32,
) -> Result<LaunchFrame, super::LoaderError> {
    launch::prepare(entry_address, stack_origin, stack_size).map_err(|error| match error {
        launch::LaunchError::InvalidEntry => {
            super::LoaderError::V3RelocationPackage(v3::Error::InvalidExecutionOffset)
        }
        launch::LaunchError::InvalidStack => {
            super::LoaderError::V3RelocationPackage(v3::Error::RegionOverflow)
        }
    })
}

fn read_header<D>(file: &AmrnFile<'_, D>) -> Result<[u8; v3::HEADER_SIZE], super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header = [0; v3::HEADER_SIZE];
    super::read_exact(file, &mut header).map_err(super::LoaderError::Filesystem)?;
    Ok(header)
}

fn package_length(header: v3::Header) -> Option<u32> {
    u32::try_from(v3::HEADER_SIZE)
        .ok()?
        .checked_add(header.code_size)?
        .checked_add(header.data_init_size)?
        .checked_add(
            header
                .relocation_count
                .checked_mul(RELOCATION_BYTES as u32)?,
        )
}

fn validate_payload<D>(
    file: &AmrnFile<'_, D>,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let _header = read_header(file)?;
    let mut checksum = Crc32::new();
    let mut chunk: Block = [0; BLOCK_SIZE];
    read_checksum_bytes(file, header.code_size, &mut chunk, &mut checksum)?;
    read_checksum_bytes(file, header.data_init_size, &mut chunk, &mut checksum)?;
    let mut entry = [0; RELOCATION_BYTES];
    for _ in 0..header.relocation_count {
        super::read_exact(file, &mut entry).map_err(super::LoaderError::Filesystem)?;
        checksum.update(&entry);
        let relocation =
            v3::decode_relocation(&entry).map_err(super::LoaderError::V3RelocationPackage)?;
        v3::validate_relocation(header, contract, relocation)
            .map_err(super::LoaderError::V3RelocationPackage)?;
    }
    if checksum.finish() != header.crc32 {
        return Err(super::LoaderError::V3RelocationPackage(
            v3::Error::CrcMismatch,
        ));
    }
    Ok(())
}

fn read_checksum_bytes<D>(
    file: &AmrnFile<'_, D>,
    size: u32,
    chunk: &mut Block,
    checksum: &mut Crc32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut remaining = usize::try_from(size)
        .map_err(|_| super::LoaderError::V3RelocationPackage(v3::Error::InvalidPayload))?;
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        super::read_exact(file, &mut chunk[..chunk_size])
            .map_err(super::LoaderError::Filesystem)?;
        checksum.update(&chunk[..chunk_size]);
        remaining -= chunk_size;
    }
    Ok(())
}

fn copy_segments_and_relocate<D>(
    file: &AmrnFile<'_, D>,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError>
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
        .ok_or(super::LoaderError::V3RelocationPackage(
            v3::Error::AddressOverflow,
        ))?;
    zero_segment(zero_start, header.data_zero_size)?;
    apply_relocations(file, header, contract)
}

fn apply_relocations<D>(
    file: &AmrnFile<'_, D>,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let code = mutable_segment(header.code_load_address, header.code_size)?;
    let data = mutable_segment(header.data_load_address, header.data_init_size)?;
    let mut entry = [0; RELOCATION_BYTES];
    for _ in 0..header.relocation_count {
        super::read_exact(file, &mut entry).map_err(super::LoaderError::Filesystem)?;
        let relocation =
            v3::decode_relocation(&entry).map_err(super::LoaderError::V3RelocationPackage)?;
        v3::apply(code, data, header, contract, &[relocation])
            .map_err(super::LoaderError::V3RelocationPackage)?;
    }
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
        .map_err(|_| super::LoaderError::V3RelocationPackage(v3::Error::InvalidPayload))?;
    let mut offset = 0usize;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        super::read_exact(file, &mut chunk[..chunk_size])
            .map_err(super::LoaderError::Filesystem)?;
        let address = usize::try_from(destination)
            .ok()
            .and_then(|value| value.checked_add(offset))
            .ok_or(super::LoaderError::V3RelocationPackage(
                v3::Error::AddressOverflow,
            ))?;
        let target = unsafe {
            // SAFETY: format validation proved this copied segment fits its target SRAM region.
            core::slice::from_raw_parts_mut(address as *mut u8, chunk_size)
        };
        target.copy_from_slice(&chunk[..chunk_size]);
        offset += chunk_size;
        remaining -= chunk_size;
    }
    Ok(())
}

fn mutable_segment(address: u32, size: u32) -> Result<&'static mut [u8], super::LoaderError> {
    let length = usize::try_from(size)
        .map_err(|_| super::LoaderError::V3RelocationPackage(v3::Error::InvalidPayload))?;
    let target = unsafe {
        // SAFETY: format validation proved the complete segment fits the selected SRAM region.
        core::slice::from_raw_parts_mut(address as *mut u8, length)
    };
    Ok(target)
}

fn zero_segment(destination: u32, size: u32) -> Result<(), super::LoaderError> {
    let target = mutable_segment(destination, size)?;
    target.fill(0);
    Ok(())
}
