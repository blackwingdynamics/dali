//! Bounded streaming loader for authenticated AMRN format 5 cartridges.

use dali_amrn::{Crc32, v3, v5};

use crate::security::launch::{self, LaunchFrame};
use crate::{
    drivers::{BLOCK_SIZE, Block},
    loader::{CartridgeReader, read_cartridge_exact},
    runtime::{
        application::lifecycle::{ApplicationIdentity, ApplicationLifecycle},
        memory::slots::SlotAllocation,
    },
};

use super::execution::LoadedApplication;

/// Size of the authenticated signature envelope in the v5 format.
const SIGNATURE_BYTES: usize = v5::SIGNATURE_SIZE;

#[cfg(not(feature = "repository-loader"))]
pub(crate) fn load_file<R>(
    file: R,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    R: CartridgeReader,
{
    load_file_with_key(file, slot_manager, None)
}

/// Loads a cartridge whose signing key was authorized by the repository chain.
#[cfg(feature = "repository-loader")]
pub(crate) fn load_file_with_public_key<R>(
    file: R,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    public_key: &[u8; dali_crypto::PUBLIC_KEY_LENGTH],
) -> Result<LoadedApplication, super::LoaderError>
where
    R: CartridgeReader,
{
    load_file_with_key(file, slot_manager, Some(public_key))
}

/// Loads and authenticates a file using an optional repository-authorized key.
fn load_file_with_key<R>(
    mut file: R,
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
    repository_key: Option<&[u8; dali_crypto::PUBLIC_KEY_LENGTH]>,
) -> Result<LoadedApplication, super::LoaderError>
where
    R: CartridgeReader,
{
    let header_bytes = read_header(&mut file)?;
    let signed_size = read_u32(&header_bytes, v5::SIGNED_SIZE_OFFSET);
    let envelope = read_envelope(&mut file, signed_size)?;
    let (header, contract, allocation) = select_header(&header_bytes, &envelope, slot_manager)?;
    if !super::supports_required_services(header.metadata.required_services) {
        return Err(super::LoaderError::UnsupportedServices(
            header.metadata.required_services,
        ));
    }
    let public_key = repository_key
        .copied()
        .or_else(|| {
            crate::platform::trust_anchors()
                .iter()
                .find(|anchor| header.signature.key_id == anchor.key_id)
                .map(|anchor| anchor.public_key)
        })
        .ok_or(super::LoaderError::UnknownTrustAnchor)?;
    validate_cartridge(&mut file, header, contract, signed_size, &public_key)?;
    file.rewind()?;
    let _header = read_header(&mut file)?;
    copy_segments_and_relocate(&mut file, header.image, contract)?;
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
        ApplicationIdentity::new(header.metadata.cartridge_id),
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

/// Reads the fixed v5 header from the current file position.
fn read_header<R: CartridgeReader>(
    file: &mut R,
) -> Result<[u8; v5::HEADER_SIZE], super::LoaderError> {
    let mut header = [0; v5::HEADER_SIZE];
    read_cartridge_exact(file, &mut header)?;
    Ok(header)
}

/// Reads the signature envelope after validating its file bounds.
fn read_envelope<R: CartridgeReader>(
    file: &mut R,
    signed_size: u32,
) -> Result<[u8; SIGNATURE_BYTES], super::LoaderError> {
    if u64::from(signed_size) + SIGNATURE_BYTES as u64 > u64::from(file.length()) {
        return Err(v5_error(v5::Error::InvalidSignature));
    }
    file.rewind()?;
    skip_bytes(file, signed_size)?;
    let mut envelope = [0; SIGNATURE_BYTES];
    read_cartridge_exact(file, &mut envelope)?;
    Ok(envelope)
}

/// Consumes a bounded number of bytes from the cartridge stream.
fn skip_bytes<R: CartridgeReader>(file: &mut R, size: u32) -> Result<(), super::LoaderError> {
    let mut remaining = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let mut chunk = [0; BLOCK_SIZE];
    while remaining > 0 {
        let size = remaining.min(chunk.len());
        read_cartridge_exact(file, &mut chunk[..size])?;
        remaining -= size;
    }
    Ok(())
}

/// Selects the header and manifest slot matching the current target.
fn select_header<'a>(
    bytes: &[u8; v5::HEADER_SIZE],
    envelope: &'a [u8; SIGNATURE_BYTES],
    slot_manager: &mut crate::runtime::memory::slots::SlotManager,
) -> Result<(v5::Header<'a>, v3::Contract, SlotAllocation), super::LoaderError> {
    let target =
        crate::platform::target_profile().ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
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

/// Validates cartridge sizes, CRCs, and the authenticated payload stream.
fn validate_cartridge<R: CartridgeReader>(
    file: &mut R,
    header: v5::Header<'_>,
    contract: v3::Contract,
    signed_size: u32,
    public_key: &[u8; dali_crypto::PUBLIC_KEY_LENGTH],
) -> Result<(), super::LoaderError> {
    let expected_signed_size = expected_signed_size(header.image)?;
    if signed_size != expected_signed_size
        || header.image.relocation_offset
            != v5::HEADER_SIZE as u32 + header.image.code_size + header.image.data_init_size
        || u64::from(file.length()) != u64::from(signed_size) + SIGNATURE_BYTES as u64
    {
        return Err(v5_error(v5::Error::InvalidPayload));
    }
    let signature: [u8; 64] = header
        .signature
        .signature
        .try_into()
        .map_err(|_| v5_error(v5::Error::InvalidSignature))?;
    let mut verifier = dali_crypto::begin_verify(public_key, &signature)
        .map_err(super::LoaderError::SignatureVerification)?;
    file.rewind()?;
    let mut header_bytes = [0; v5::HEADER_SIZE];
    read_cartridge_exact(file, &mut header_bytes)?;
    verifier.update(&header_bytes);
    let mut cartridge_crc = Crc32::new();
    cartridge_crc.update(&header_bytes[..v5::CARTRIDGE_CRC32_OFFSET]);
    cartridge_crc.update(&header_bytes[v5::CARTRIDGE_CRC32_OFFSET + 4..]);
    let mut payload_crc = Crc32::new();
    read_payload(
        file,
        header.image.code_size,
        &mut verifier,
        &mut payload_crc,
        &mut cartridge_crc,
    )?;
    read_payload(
        file,
        header.image.data_init_size,
        &mut verifier,
        &mut payload_crc,
        &mut cartridge_crc,
    )?;
    read_relocations(
        file,
        header.image,
        contract,
        &mut verifier,
        &mut payload_crc,
        &mut cartridge_crc,
    )?;
    if cartridge_crc.finish() != header.cartridge_crc32
        || payload_crc.finish() != header.image.crc32
    {
        return Err(v5_error(v5::Error::CrcMismatch));
    }
    verifier
        .finalize()
        .map_err(super::LoaderError::SignatureVerification)
}

/// Computes the signed byte length from the decoded image header.
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

/// Streams one payload segment through authentication and CRC state.
fn read_payload<R: CartridgeReader>(
    file: &mut R,
    size: u32,
    verifier: &mut dali_crypto::StreamingVerifier,
    payload_crc: &mut Crc32,
    cartridge_crc: &mut Crc32,
) -> Result<(), super::LoaderError> {
    let mut remaining = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let size = remaining.min(chunk.len());
        read_cartridge_exact(file, &mut chunk[..size])?;
        verifier.update(&chunk[..size]);
        payload_crc.update(&chunk[..size]);
        cartridge_crc.update(&chunk[..size]);
        remaining -= size;
    }
    Ok(())
}

/// Streams and validates all relocation records in the cartridge.
fn read_relocations<R: CartridgeReader>(
    file: &mut R,
    header: v3::Header,
    contract: v3::Contract,
    verifier: &mut dali_crypto::StreamingVerifier,
    payload_crc: &mut Crc32,
    cartridge_crc: &mut Crc32,
) -> Result<(), super::LoaderError> {
    let mut entry = [0; v3::RELOCATION_ENTRY_SIZE];
    for _ in 0..header.relocation_count {
        read_cartridge_exact(file, &mut entry)?;
        verifier.update(&entry);
        payload_crc.update(&entry);
        cartridge_crc.update(&entry);
        let relocation =
            v3::decode_relocation(&entry).map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
        v3::validate_relocation(header, contract, relocation)
            .map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
    }
    Ok(())
}

/// Converts validated image addresses into the launch frame.
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

/// Reads a little-endian word from a validated v5 header offset.
fn read_u32(bytes: &[u8; v5::HEADER_SIZE], offset: usize) -> u32 {
    let mut value = [0; 4];
    value.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(value)
}

/// Wraps a v5 format error in the pipeline error type.
fn v5_error(error: v5::Error) -> super::LoaderError {
    super::LoaderError::V5SignedCartridge(error)
}

/// Copies validated segments and applies validated relocations.
fn copy_segments_and_relocate<R: CartridgeReader>(
    file: &mut R,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError> {
    file.rewind()?;
    let mut header_bytes = [0; v5::HEADER_SIZE];
    read_cartridge_exact(file, &mut header_bytes)?;
    copy_segment(file, header.code_load_address, header.code_size)?;
    copy_segment(file, header.data_load_address, header.data_init_size)?;
    let zero_start = header
        .data_load_address
        .checked_add(header.data_init_size)
        .ok_or(v5_error(v5::Error::InvalidPayload))?;
    zero_segment(zero_start, header.data_zero_size)?;
    apply_relocations(file, header, contract)
}

/// Applies each validated relocation to the copied image.
fn apply_relocations<R: CartridgeReader>(
    file: &mut R,
    header: v3::Header,
    contract: v3::Contract,
) -> Result<(), super::LoaderError> {
    let code = mutable_segment(header.code_load_address, header.code_size)?;
    let data = mutable_segment(header.data_load_address, header.data_init_size)?;
    let mut entry = [0; v3::RELOCATION_ENTRY_SIZE];
    for _ in 0..header.relocation_count {
        read_cartridge_exact(file, &mut entry)?;
        let relocation =
            v3::decode_relocation(&entry).map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
        v3::apply(code, data, header, contract, &[relocation])
            .map_err(|_| v5_error(v5::Error::InvalidRelocation))?;
    }
    Ok(())
}

/// Copies one validated image segment into its target address.
fn copy_segment<R: CartridgeReader>(
    file: &mut R,
    destination: u32,
    size: u32,
) -> Result<(), super::LoaderError> {
    let mut remaining = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let mut offset = 0usize;
    let mut chunk: Block = [0; BLOCK_SIZE];
    while remaining > 0 {
        let chunk_size = remaining.min(chunk.len());
        read_cartridge_exact(file, &mut chunk[..chunk_size])?;
        let address = usize::try_from(destination)
            .ok()
            .and_then(|value| value.checked_add(offset))
            .ok_or(v5_error(v5::Error::InvalidPayload))?;
        let target = unsafe {
            // SAFETY: v5 validation proved this copied segment fits its manifest slot.
            core::slice::from_raw_parts_mut(address as *mut u8, chunk_size)
        };
        target.copy_from_slice(&chunk[..chunk_size]);
        offset += chunk_size;
        remaining -= chunk_size;
    }
    Ok(())
}

/// Returns a mutable slice for a segment already proven to be in bounds.
fn mutable_segment(address: u32, size: u32) -> Result<&'static mut [u8], super::LoaderError> {
    let length = usize::try_from(size).map_err(|_| v5_error(v5::Error::InvalidPayload))?;
    let target = unsafe {
        // SAFETY: v5 validation proved the complete segment fits the selected manifest slot.
        core::slice::from_raw_parts_mut(address as *mut u8, length)
    };
    Ok(target)
}

/// Clears the validated zero-initialized image segment.
fn zero_segment(destination: u32, size: u32) -> Result<(), super::LoaderError> {
    let target = mutable_segment(destination, size)?;
    target.fill(0);
    Ok(())
}
