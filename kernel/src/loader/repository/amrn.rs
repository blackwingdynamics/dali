//! Generic streamed validation for signed AMRN v5 packages.

use dali_amrn::{Crc32, v3, v5};
use dali_metadata::{DelegationMetadata, Sha256Digest};

use crate::storage::repository::{RepositoryPackageDigest, RepositoryStreamStorage};

/// Caller-owned fixed storage required by the AMRN two-pass verifier.
pub(crate) struct AmrnStreamBuffers {
    /// Fixed AMRN header retained between passes.
    pub header: [u8; v5::HEADER_SIZE],
    /// Fixed DSIG trailer retained between passes.
    pub signature: [u8; v5::SIGNATURE_SIZE],
}

impl AmrnStreamBuffers {
    /// Creates empty AMRN stream state.
    pub const fn new() -> Self {
        Self {
            header: [0; v5::HEADER_SIZE],
            signature: [0; v5::SIGNATURE_SIZE],
        }
    }
}

impl Default for AmrnStreamBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors returned by the generic streamed AMRN validator.
#[derive(Debug)]
pub(crate) enum AmrnStreamError<E> {
    /// Repository storage failed while producing a package chunk.
    Storage(E),
    /// The adapter length did not match the delivered bytes.
    LengthMismatch,
    /// The package header or trailer was malformed.
    InvalidHeader(dali_amrn::v5::Error),
    /// The package digest did not match the target record.
    DigestMismatch,
    /// The package signature did not match the delegated developer key.
    InvalidSignature,
    /// The package CRC or payload CRC was invalid.
    InvalidCrc,
}

/// Streams and validates one AMRN v5 package in two bounded passes.
#[inline(never)]
pub(crate) fn verify_streamed_amrn<'a, S>(
    storage: &mut S,
    digest: RepositoryPackageDigest,
    delegation: &DelegationMetadata,
    contract: v3::Contract,
    chunk: &mut [u8],
    buffers: &'a mut AmrnStreamBuffers,
) -> Result<v5::Header<'a>, AmrnStreamError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let total = capture_package_shape(storage, digest, chunk, buffers)?;
    let header = v5::parse_header_parts(&buffers.header, &buffers.signature, contract)
        .map_err(AmrnStreamError::InvalidHeader)?;
    let signed_size = usize::try_from(read_u32(
        &buffers.header[v5::SIGNED_SIZE_OFFSET..v5::SIGNED_SIZE_OFFSET + 4],
    ))
    .map_err(|_| AmrnStreamError::InvalidHeader(v5::Error::InvalidHeader))?;
    if signed_size + v5::SIGNATURE_SIZE != total {
        return Err(AmrnStreamError::InvalidHeader(v5::Error::InvalidSignature));
    }
    if header.signature.key_id != delegation.key_id.0 {
        return Err(AmrnStreamError::InvalidSignature);
    }
    let signature: [u8; dali_crypto::SIGNATURE_LENGTH] = header
        .signature
        .signature
        .try_into()
        .map_err(|_| AmrnStreamError::InvalidSignature)?;
    let mut verifier = dali_crypto::begin_verify(&delegation.public_key.0, &signature)
        .map_err(|_| AmrnStreamError::InvalidSignature)?;
    let mut package_crc = Crc32::new();
    package_crc.update(&buffers.header[..v5::PACKAGE_CRC32_OFFSET]);
    package_crc.update(&buffers.header[v5::PACKAGE_CRC32_OFFSET + 4..]);
    let mut payload_crc = Crc32::new();
    replay_package(
        storage,
        digest,
        chunk,
        signed_size,
        &mut verifier,
        &mut package_crc,
        &mut payload_crc,
    )?;
    if package_crc.finish() != header.package_crc32 || payload_crc.finish() != header.image.crc32 {
        return Err(AmrnStreamError::InvalidCrc);
    }
    verifier
        .finalize()
        .map_err(|_| AmrnStreamError::InvalidSignature)?;
    Ok(header)
}

#[inline(never)]
fn capture_package_shape<S>(
    storage: &mut S,
    digest: RepositoryPackageDigest,
    chunk: &mut [u8],
    buffers: &mut AmrnStreamBuffers,
) -> Result<usize, AmrnStreamError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut offset = 0_usize;
    let mut package_digest = dali_crypto::Sha256Accumulator::new();
    let mut tail = TailBuffer::new();
    let total = storage
        .stream_package(digest, chunk, |bytes| {
            package_digest.update(bytes);
            copy_header(bytes, &mut offset, &mut buffers.header);
            tail.push(bytes);
            offset = offset.saturating_add(bytes.len());
            Ok(())
        })
        .map_err(AmrnStreamError::Storage)?;
    if usize::try_from(total).ok() != Some(offset) || tail.length != v5::SIGNATURE_SIZE {
        return Err(AmrnStreamError::LengthMismatch);
    }
    buffers.signature.copy_from_slice(&tail.bytes);
    let actual = Sha256Digest(package_digest.finalize());
    if actual.0 != digest.0 {
        return Err(AmrnStreamError::DigestMismatch);
    }
    Ok(offset)
}

#[inline(never)]
fn replay_package<S>(
    storage: &mut S,
    digest: RepositoryPackageDigest,
    chunk: &mut [u8],
    signed_size: usize,
    verifier: &mut dali_crypto::StreamingVerifier,
    package_crc: &mut Crc32,
    payload_crc: &mut Crc32,
) -> Result<(), AmrnStreamError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut offset = 0_usize;
    storage
        .stream_package(digest, chunk, |bytes| {
            let signed_end = signed_size.min(offset.saturating_add(bytes.len()));
            if offset < signed_end {
                let signed = &bytes[..signed_end - offset];
                verifier.update(signed);
                if offset >= v5::HEADER_SIZE {
                    package_crc.update(signed);
                    payload_crc.update(signed);
                } else if offset + signed.len() > v5::HEADER_SIZE {
                    let payload_start = v5::HEADER_SIZE - offset;
                    package_crc.update(&signed[payload_start..]);
                    payload_crc.update(&signed[payload_start..]);
                }
            }
            offset = offset.saturating_add(bytes.len());
            Ok(())
        })
        .map_err(AmrnStreamError::Storage)?;
    if offset != signed_size + v5::SIGNATURE_SIZE {
        return Err(AmrnStreamError::LengthMismatch);
    }
    Ok(())
}

fn copy_header(bytes: &[u8], offset: &mut usize, header: &mut [u8; v5::HEADER_SIZE]) {
    if *offset < header.len() {
        let count = (header.len() - *offset).min(bytes.len());
        header[*offset..*offset + count].copy_from_slice(&bytes[..count]);
    }
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

struct TailBuffer {
    bytes: [u8; v5::SIGNATURE_SIZE],
    length: usize,
}
impl TailBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; v5::SIGNATURE_SIZE],
            length: 0,
        }
    }
    fn push(&mut self, bytes: &[u8]) {
        if bytes.len() >= self.bytes.len() {
            let length = self.bytes.len();
            self.bytes.copy_from_slice(&bytes[bytes.len() - length..]);
            self.length = length;
            return;
        }
        let keep = self.length.min(self.bytes.len() - bytes.len());
        self.bytes.copy_within(self.length - keep..self.length, 0);
        self.bytes[keep..keep + bytes.len()].copy_from_slice(bytes);
        self.length = keep + bytes.len();
    }
}
