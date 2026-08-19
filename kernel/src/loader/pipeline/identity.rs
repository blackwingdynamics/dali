//! Bounded streaming loader for AMRN format 4 identity packages.

use dali_amrn::{v3, v4};

use crate::security::launch::{self, LaunchFrame};
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    runtime::{
        application::lifecycle::{ApplicationIdentity, ApplicationLifecycle},
        memory::slots::SlotAllocation,
    },
    storage::filesystem::AmrnFile,
};

use super::execution::LoadedApplication;

const RELOCATION_BYTES: usize = v4::RELOCATION_ENTRY_SIZE;
const SINGLE_PACKAGE_CATALOG_CAPACITY: usize = 1;

pub(crate) fn load_file<D>(
    file: AmrnFile<'_, D>,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let header = read_header(&file)?;
    let (header, contract, allocation) = parse_target_header(&header, slot_manager)?;
    if !super::supports_required_services(header.metadata.required_services) {
        return Err(super::LoaderError::UnsupportedServices(
            header.metadata.required_services,
        ));
    }
    let relocation_offset = u32::try_from(v4::HEADER_SIZE)
        .ok()
        .and_then(|offset| offset.checked_add(header.image.code_size))
        .and_then(|offset| offset.checked_add(header.image.data_init_size))
        .ok_or(v4_error(v4::Error::InvalidPayload))?;
    if header.image.relocation_offset != relocation_offset {
        return Err(v4_error(v4::Error::InvalidPayload));
    }
    let expected_length =
        package_length(header.image).ok_or(v4_error(v4::Error::InvalidPayload))?;
    if u64::from(file.length()) != u64::from(expected_length) {
        return Err(v4_error(v4::Error::InvalidPayload));
    }
    validate_payload(&file, header, contract)?;
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let _header = read_header(&file)?;
    copy_segments_and_relocate(&file, header.image, contract)?;
    let entry_address = header
        .image
        .code_load_address
        .checked_add(header.image.execution_offset)
        .ok_or(v4_error(v4::Error::InvalidPayload))?;
    let stack_origin = header
        .image
        .data_load_address
        .checked_add(header.image.data_init_size)
        .and_then(|address| address.checked_add(header.image.data_zero_size))
        .ok_or(v4_error(v4::Error::InvalidPayload))?;
    let launch_frame = prepare_launch(entry_address, stack_origin, header.image.stack_size)?;
    launch::materialize(launch_frame);
    let mut lifecycle = ApplicationLifecycle::discovered(
        ApplicationIdentity::new(header.metadata.package_id),
        allocation.slot().id,
    );
    lifecycle
        .record_allocation(allocation)
        .map_err(super::LoaderError::Lifecycle)?;
    Ok(LoadedApplication {
        entry_address,
        psp_top: stack_origin
            .checked_add(header.image.stack_size)
            .ok_or(v4_error(v4::Error::InvalidPayload))?,
        launch_frame,
        slot: allocation.slot(),
        allocation,
        lifecycle: Some(lifecycle),
    })
}

pub(super) fn parse_target_header(
    bytes: &[u8; v4::HEADER_SIZE],
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<(v4::Header, v3::Contract, SlotAllocation), super::LoaderError> {
    let target = crate::platform::TARGET_PROFILE;
    let isolation = target
        .memory
        .isolation
        .ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
    let (header, contract, slot) =
        crate::loader_contract::select_slot(bytes, target.amrn_target_id, isolation.slots)
            .map_err(|_| v4_error(v4::Error::InvalidHeader))?;
    let mut catalog =
        crate::loader_contract::PackageCatalog::<SINGLE_PACKAGE_CATALOG_CAPACITY>::new();
    catalog
        .register(header, slot, slot_manager.is_reserved(slot))
        .map_err(super::LoaderError::PackageCatalog)?;
    let selected: crate::loader_contract::DiscoveredPackage = catalog.select().ok_or(
        super::LoaderError::PackageCatalog(crate::loader_contract::CatalogError::CapacityExceeded),
    )?;
    let allocation = slot_manager
        .reserve(selected.slot)
        .map_err(super::LoaderError::SlotManager)?;
    Ok((selected.header, contract, allocation))
}

fn prepare_launch(
    entry_address: u32,
    stack_origin: u32,
    stack_size: u32,
) -> Result<LaunchFrame, super::LoaderError> {
    launch::prepare(entry_address, stack_origin, stack_size).map_err(|error| match error {
        launch::LaunchError::InvalidEntry => v4_error(v4::Error::InvalidPayload),
        launch::LaunchError::InvalidStack => v4_error(v4::Error::InvalidPayload),
    })
}

pub(crate) fn read_header<D>(
    file: &AmrnFile<'_, D>,
) -> Result<[u8; v4::HEADER_SIZE], super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header = [0; v4::HEADER_SIZE];
    super::read_exact(file, &mut header).map_err(super::LoaderError::Filesystem)?;
    Ok(header)
}

fn package_length(header: v3::Header) -> Option<u32> {
    u32::try_from(v4::HEADER_SIZE)
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
    header: v4::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let reader = FileReader { file };
    crate::loader_contract::validate_stream(&reader, header, contract).map_err(map_stream_error)
}

pub(super) fn copy_segments_and_relocate<D>(
    file: &AmrnFile<'_, D>,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    copy_segment(file, header.code_load_address, header.code_size)?;
    copy_segment(file, header.data_load_address, header.data_init_size)?;
    let zero_start = header
        .data_load_address
        .checked_add(header.data_init_size)
        .ok_or(v4_error(v4::Error::InvalidPayload))?;
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
        let relocation = v3::decode_relocation(&entry).map_err(relocation_error)?;
        v3::apply(code, data, header, contract, &[relocation]).map_err(relocation_error)?;
    }
    Ok(())
}

pub(super) fn copy_segment<D>(
    file: &AmrnFile<'_, D>,
    destination: u32,
    size: u32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut remaining = usize::try_from(size).map_err(|_| v4_error(v4::Error::InvalidPayload))?;
    let mut offset = 0usize;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        super::read_exact(file, &mut chunk[..chunk_size])
            .map_err(super::LoaderError::Filesystem)?;
        let address = usize::try_from(destination)
            .ok()
            .and_then(|value| value.checked_add(offset))
            .ok_or(v4_error(v4::Error::InvalidPayload))?;
        let target = unsafe {
            // SAFETY: v4 validation proved this segment fits its manifest slot.
            core::slice::from_raw_parts_mut(address as *mut u8, chunk_size)
        };
        target.copy_from_slice(&chunk[..chunk_size]);
        offset += chunk_size;
        remaining -= chunk_size;
    }
    Ok(())
}

pub(super) fn mutable_segment(
    address: u32,
    size: u32,
) -> Result<&'static mut [u8], super::LoaderError> {
    let length = usize::try_from(size).map_err(|_| v4_error(v4::Error::InvalidPayload))?;
    let target = unsafe {
        // SAFETY: v4 validation proved the complete segment fits its manifest slot.
        core::slice::from_raw_parts_mut(address as *mut u8, length)
    };
    Ok(target)
}

pub(super) fn zero_segment(destination: u32, size: u32) -> Result<(), super::LoaderError> {
    let target = mutable_segment(destination, size)?;
    target.fill(0);
    Ok(())
}

pub(super) struct FileReader<'a, 'file, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    file: &'a AmrnFile<'file, D>,
}

impl<'a, 'file, D> crate::loader_contract::PackageReader for FileReader<'a, 'file, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    type Error = embedded_sdmmc::Error<StorageError>;

    fn read(&self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.file.read(buffer)
    }

    fn rewind(&self) -> Result<(), Self::Error> {
        self.file.rewind()
    }
}

fn map_stream_error(
    error: crate::loader_contract::StreamError<embedded_sdmmc::Error<StorageError>>,
) -> super::LoaderError {
    match error {
        crate::loader_contract::StreamError::Read(error) => super::LoaderError::Filesystem(error),
        crate::loader_contract::StreamError::InvalidRelocation => {
            v4_error(v4::Error::InvalidRelocation)
        }
        crate::loader_contract::StreamError::CrcMismatch => v4_error(v4::Error::CrcMismatch),
    }
}

pub(crate) fn v4_error(error: v4::Error) -> super::LoaderError {
    super::LoaderError::V4IdentityPackage(error)
}

fn relocation_error(_: v3::Error) -> super::LoaderError {
    v4_error(v4::Error::InvalidRelocation)
}
