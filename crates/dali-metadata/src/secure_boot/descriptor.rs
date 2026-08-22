//! Fixed Secure Boot kernel-image descriptor codec.

use crate::{BoundedText, Sha256Digest};
use sha2::{Digest, Sha256};

const DESCRIPTOR_MAGIC: [u8; 4] = *b"DLKB";
const DESCRIPTOR_VERSION: u8 = 1;
const TARGET_LENGTH_OFFSET: usize = DESCRIPTOR_MAGIC.len() + 1;
pub(crate) const TARGET_BYTES_OFFSET: usize = TARGET_LENGTH_OFFSET + core::mem::size_of::<u16>();
const VERSION_OFFSET: usize = TARGET_BYTES_OFFSET + crate::MAX_TARGET_PROFILE_BYTES;
const LENGTH_OFFSET: usize = VERSION_OFFSET + core::mem::size_of::<u64>();
const DIGEST_OFFSET: usize = LENGTH_OFFSET + core::mem::size_of::<u32>();
/// Fixed encoded length signed for one kernel-image descriptor.
pub const SECURE_BOOT_DESCRIPTOR_BYTES: usize = DIGEST_OFFSET + crate::SHA256_LENGTH;

/// Canonical metadata describing one kernel image before it is entered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelImageDescriptor {
    /// Target profile authorized to execute the image.
    pub target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
    /// Monotonic kernel image version.
    pub version: u64,
    /// Exact image byte length.
    pub length: u32,
    /// SHA-256 digest of the complete image.
    pub sha256: Sha256Digest,
}

impl KernelImageDescriptor {
    /// Creates a descriptor from the exact image bytes to be verified.
    pub fn new(
        target_profile: BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>,
        version: u64,
        image: &[u8],
    ) -> Option<Self> {
        Some(Self {
            target_profile,
            version: (version != 0).then_some(version)?,
            length: u32::try_from(image.len())
                .ok()
                .filter(|length| *length != 0)?,
            sha256: Sha256Digest(Sha256::digest(image).into()),
        })
    }

    /// Encodes the fixed descriptor bytes covered by the root signature.
    pub fn encode(&self) -> [u8; SECURE_BOOT_DESCRIPTOR_BYTES] {
        let mut output = [0; SECURE_BOOT_DESCRIPTOR_BYTES];
        output[..DESCRIPTOR_MAGIC.len()].copy_from_slice(&DESCRIPTOR_MAGIC);
        output[DESCRIPTOR_MAGIC.len()] = DESCRIPTOR_VERSION;
        let target = self.target_profile.as_str().unwrap_or("").as_bytes();
        output[TARGET_LENGTH_OFFSET..TARGET_BYTES_OFFSET]
            .copy_from_slice(&(target.len() as u16).to_le_bytes());
        output[TARGET_BYTES_OFFSET..TARGET_BYTES_OFFSET + target.len()].copy_from_slice(target);
        output[VERSION_OFFSET..LENGTH_OFFSET].copy_from_slice(&self.version.to_le_bytes());
        output[LENGTH_OFFSET..DIGEST_OFFSET].copy_from_slice(&self.length.to_le_bytes());
        output[DIGEST_OFFSET..].copy_from_slice(&self.sha256.0);
        output
    }

    /// Decodes one exact descriptor and rejects non-zero padding or fields.
    pub fn decode(input: &[u8]) -> Result<Self, SecureBootDescriptorError> {
        if input.len() != SECURE_BOOT_DESCRIPTOR_BYTES {
            return Err(SecureBootDescriptorError::InvalidLength);
        }
        if input[..DESCRIPTOR_MAGIC.len()] != DESCRIPTOR_MAGIC {
            return Err(SecureBootDescriptorError::InvalidMagic);
        }
        if input[DESCRIPTOR_MAGIC.len()] != DESCRIPTOR_VERSION {
            return Err(SecureBootDescriptorError::UnsupportedVersion);
        }
        let target_length = usize::from(u16::from_le_bytes(
            input[TARGET_LENGTH_OFFSET..TARGET_BYTES_OFFSET]
                .try_into()
                .map_err(|_| SecureBootDescriptorError::InvalidTarget)?,
        ));
        if target_length == 0 || target_length > crate::MAX_TARGET_PROFILE_BYTES {
            return Err(SecureBootDescriptorError::InvalidTarget);
        }
        let target_bytes = &input[TARGET_BYTES_OFFSET..VERSION_OFFSET];
        if target_bytes[target_length..].iter().any(|byte| *byte != 0) {
            return Err(SecureBootDescriptorError::InvalidTarget);
        }
        let target = core::str::from_utf8(&target_bytes[..target_length])
            .ok()
            .and_then(|value| BoundedText::new(value).ok())
            .ok_or(SecureBootDescriptorError::InvalidTarget)?;
        let version = u64::from_le_bytes(
            input[VERSION_OFFSET..LENGTH_OFFSET]
                .try_into()
                .map_err(|_| SecureBootDescriptorError::InvalidVersion)?,
        );
        let length = u32::from_le_bytes(
            input[LENGTH_OFFSET..DIGEST_OFFSET]
                .try_into()
                .map_err(|_| SecureBootDescriptorError::InvalidLength)?,
        );
        let sha256 = Sha256Digest(
            input[DIGEST_OFFSET..]
                .try_into()
                .map_err(|_| SecureBootDescriptorError::InvalidDigest)?,
        );
        if version == 0 {
            return Err(SecureBootDescriptorError::InvalidVersion);
        }
        if length == 0 {
            return Err(SecureBootDescriptorError::InvalidLength);
        }
        if sha256.0 == [0; crate::SHA256_LENGTH] {
            return Err(SecureBootDescriptorError::InvalidDigest);
        }
        Ok(Self {
            target_profile: target,
            version,
            length,
            sha256,
        })
    }
}

/// Errors returned by the fixed Secure Boot descriptor codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecureBootDescriptorError {
    /// The input length is not the fixed descriptor length.
    InvalidLength,
    /// The descriptor magic is not `DLKB`.
    InvalidMagic,
    /// The descriptor version is unsupported.
    UnsupportedVersion,
    /// The target profile bytes or padding are invalid.
    InvalidTarget,
    /// The image version is zero.
    InvalidVersion,
    /// The image digest is empty.
    InvalidDigest,
}
