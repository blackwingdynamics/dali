//! Bounded streaming loader for authenticated AMRN format 5 packages.

use dali_amrn::{Crc32, v3, v5};

use crate::security::launch::{self, LaunchFrame};
use crate::{
    drivers::{BLOCK_SIZE, Block, StorageError},
    runtime::{
        application::lifecycle::{ApplicationIdentity, ApplicationLifecycle},
        memory::slots::SlotAllocation,
    },
    storage::filesystem::AmrnFile,
};

use super::{execution::LoadedApplication, identity};

const SIGNATURE_BYTES: usize = v5::SIGNATURE_SIZE;

pub(crate) fn load_file<D>(
    file: AmrnFile<'_, D>,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let header_bytes = read_header(&file)?;
    let signed_size = read_u32(&header_bytes, v5::SIGNED_SIZE_OFFSET);
    let envelope = read_envelope(&file, signed_size)?;
    let (header, contract, allocation) = select_header(&header_bytes, &envelope, slot_manager)?;
    validate_package(&file, header, contract, signed_size)?;
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let _header = read_header(&file)?;
    identity::copy_segments_and_relocate(&file, header.image, contract)?;
    let entry_address = header
        .image
        .code_load_address
        .checked_add(header.image.execution_offset)
        .ok_or(v5_error(v5::Error::InvalidPayload))?;
    let stack_origin = header
        .image
        .data_load_address
        .checked_add(header.image.data_init_size)
        .and_then(|address| address.checked_add(header.image.data_zero_size))
        .ok_or(v5_error(v5::Error::InvalidPayload))?;
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
            .ok_or(v5_error(v5::Error::InvalidPayload))?,
        launch_frame,
        slot: allocation.slot(),
        allocation,
        lifecycle: Some(lifecycle),
    })
}

fn read_header<D>(file: &AmrnFile<'_, D>) -> Result<[u8; v5::HEADER_SIZE], super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut header = [0; v5::HEADER_SIZE];
    super::read_exact(file, &mut header).map_err(super::LoaderError::Filesystem)?;
    Ok(header)
}

fn read_envelope<D>(
    file: &AmrnFile<'_, D>,
    signed_size: u32,
) -> Result<[u8; SIGNATURE_BYTES], super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    if u64::from(signed_size) + SIGNATURE_BYTES as u64 > u64::from(file.length()) {
        return Err(v5_error(v5::Error::InvalidSignature));
    }
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    skip_bytes(file, signed_size)?;
    let mut envelope = [0; SIGNATURE_BYTES];
    super::read_exact(file, &mut envelope).map_err(super::LoaderError::Filesystem)?;
    Ok(envelope)
}

fn skip_bytes<D>(file: &AmrnFile<'_, D>, size: u32) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut remaining = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let mut chunk = [0; BLOCK_SIZE];
    while remaining > 0 {
        let size = remaining.min(chunk.len());
        super::read_exact(file, &mut chunk[..size]).map_err(super::LoaderError::Filesystem)?;
        remaining -= size;
    }
    Ok(())
}

fn select_header<'a>(
    bytes: &[u8; v5::HEADER_SIZE],
    envelope: &'a [u8; SIGNATURE_BYTES],
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<(v5::Header<'a>, v3::Contract, SlotAllocation), super::LoaderError> {
    let target = crate::platform::TARGET_PROFILE;
    let isolation = target
        .memory
        .isolation
        .ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
    for slot in isolation.slots.iter().copied() {
        let contract = v3::Contract {
            target_id: target.amrn_target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        if let Ok(header) = v5::parse_header_parts(bytes, envelope, contract)
            && header.metadata.slot_id == slot.id
        {
            let allocation = slot_manager
                .reserve(slot)
                .map_err(super::LoaderError::SlotManager)?;
            return Ok((header, contract, allocation));
        }
    }
    Err(v5_error(v5::Error::InvalidHeader))
}

fn validate_package<D>(
    file: &AmrnFile<'_, D>,
    header: v5::Header<'_>,
    contract: v3::Contract,
    signed_size: u32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let expected_signed_size = expected_signed_size(header.image)?;
    if signed_size != expected_signed_size
        || header.image.relocation_offset
            != v5::HEADER_SIZE as u32 + header.image.code_size + header.image.data_init_size
        || u64::from(file.length()) != u64::from(signed_size) + SIGNATURE_BYTES as u64
    {
        return Err(v5_error(v5::Error::InvalidPayload));
    }
    let anchor = crate::platform::TARGET_PROFILE
        .authentication
        .trust_anchors
        .iter()
        .find(|anchor| header.signature.key_id == anchor.key_id)
        .ok_or(super::LoaderError::UnknownTrustAnchor)?;
    let signature: [u8; 64] = header
        .signature
        .signature
        .try_into()
        .map_err(|_| v5_error(v5::Error::InvalidSignature))?;
    let mut verifier = dali_crypto::begin_verify(&anchor.public_key, &signature)
        .map_err(super::LoaderError::SignatureVerification)?;
    file.rewind().map_err(super::LoaderError::Filesystem)?;
    let mut header_bytes = [0; v5::HEADER_SIZE];
    super::read_exact(file, &mut header_bytes).map_err(super::LoaderError::Filesystem)?;
    verifier.update(&header_bytes);
    let mut package_crc = Crc32::new();
    package_crc.update(&header_bytes[..v5::PACKAGE_CRC32_OFFSET]);
    package_crc.update(&header_bytes[v5::PACKAGE_CRC32_OFFSET + 4..]);
    let mut payload_crc = Crc32::new();
    read_payload(
        file,
        header.image.code_size,
        &mut verifier,
        &mut payload_crc,
        &mut package_crc,
    )?;
    read_payload(
        file,
        header.image.data_init_size,
        &mut verifier,
        &mut payload_crc,
        &mut package_crc,
    )?;
    read_relocations(
        file,
        header.image,
        contract,
        &mut verifier,
        &mut payload_crc,
        &mut package_crc,
    )?;
    if package_crc.finish() != header.package_crc32 || payload_crc.finish() != header.image.crc32 {
        return Err(v5_error(v5::Error::CrcMismatch));
    }
    verifier
        .finalize()
        .map_err(super::LoaderError::SignatureVerification)
}

fn expected_signed_size(header: v3::Header) -> Result<u32, super::LoaderError> {
    v5::HEADER_SIZE
        .try_into()
        .ok()
        .and_then(|size: u32| {
            size.checked_add(header.code_size)
                .and_then(|size| size.checked_add(header.data_init_size))
                .and_then(|size| {
                    header
                        .relocation_count
                        .checked_mul(v3::RELOCATION_ENTRY_SIZE as u32)
                        .and_then(|relocations| size.checked_add(relocations))
                })
        })
        .ok_or(v5_error(v5::Error::InvalidPayload))
}

fn read_payload<D>(
    file: &AmrnFile<'_, D>,
    size: u32,
    verifier: &mut dali_crypto::StreamingVerifier,
    payload_crc: &mut Crc32,
    package_crc: &mut Crc32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut remaining = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let size = remaining.min(chunk.len());
        super::read_exact(file, &mut chunk[..size]).map_err(super::LoaderError::Filesystem)?;
        verifier.update(&chunk[..size]);
        payload_crc.update(&chunk[..size]);
        package_crc.update(&chunk[..size]);
        remaining -= size;
    }
    Ok(())
}

fn read_relocations<D>(
    file: &AmrnFile<'_, D>,
    header: v3::Header,
    contract: v3::Contract,
    verifier: &mut dali_crypto::StreamingVerifier,
    payload_crc: &mut Crc32,
    package_crc: &mut Crc32,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut entry = [0; v3::RELOCATION_ENTRY_SIZE];
    for _ in 0..header.relocation_count {
        super::read_exact(file, &mut entry).map_err(super::LoaderError::Filesystem)?;
        verifier.update(&entry);
        payload_crc.update(&entry);
        package_crc.update(&entry);
        let relocation =
            v3::decode_relocation(&entry).map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
        v3::validate_relocation(header, contract, relocation)
            .map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
    }
    Ok(())
}

fn prepare_launch(
    entry_address: u32,
    stack_origin: u32,
    stack_size: u32,
) -> Result<LaunchFrame, super::LoaderError> {
    launch::prepare(entry_address, stack_origin, stack_size).map_err(|error| match error {
        launch::LaunchError::InvalidEntry => v5_error(v5::Error::InvalidPayload),
        launch::LaunchError::InvalidStack => v5_error(v5::Error::InvalidPayload),
    })
}

fn read_u32(bytes: &[u8; v5::HEADER_SIZE], offset: usize) -> u32 {
    let mut value = [0; 4];
    value.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(value)
}

fn v5_error(error: v5::Error) -> super::LoaderError {
    super::LoaderError::V5SignedPackage(error)
}
