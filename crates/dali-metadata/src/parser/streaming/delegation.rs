//! Incremental Binary Metadata v2 delegation-body parser.

use crate::{
    BoundedText, DelegationMetadata, KeyId, MetadataHeader, MetadataRole, PublicKey,
    StreamingBodyError, validate_delegation,
};
use core::mem::MaybeUninit;

const QUEUE_BYTES: usize = 512;
const TEXT_BYTES: usize = crate::MAX_TARGET_PROFILE_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Version,
    Expires,
    DeveloperLength,
    Developer,
    KeyId,
    PublicKey,
    NamespaceCount,
    NamespaceLength,
    Namespace,
    TargetCount,
    TargetLength,
    Target,
    AbiCount,
    Abi,
    NotBefore,
    NotAfter,
    Complete,
}

/// Incrementally parses a developer delegation without retaining its body.
pub struct BinaryDelegationBodyStreamParser<
    const NAMESPACE_CAPACITY: usize = { crate::MAX_DELEGATION_SCOPES },
    const TARGET_CAPACITY: usize = { crate::MAX_DELEGATION_TARGETS },
    const ABI_CAPACITY: usize = { crate::MAX_DELEGATION_ABIS },
> {
    queue: ByteQueue,
    phase: Phase,
    version: u64,
    expires: u64,
    developer_id: BoundedText<{ crate::MAX_DEVELOPER_ID_BYTES }>,
    key_id: KeyId,
    public_key: PublicKey,
    namespaces: [BoundedText<{ crate::MAX_NAMESPACE_BYTES }>; NAMESPACE_CAPACITY],
    namespace_count: u8,
    namespace_index: usize,
    targets: [BoundedText<{ crate::MAX_TARGET_PROFILE_BYTES }>; TARGET_CAPACITY],
    target_count: u8,
    target_index: usize,
    abis: [u16; ABI_CAPACITY],
    abi_count: u8,
    abi_index: usize,
    text_length: usize,
    text: [u8; TEXT_BYTES],
    not_before: u64,
    not_after: u64,
}

impl<const NAMESPACE_CAPACITY: usize, const TARGET_CAPACITY: usize, const ABI_CAPACITY: usize>
    BinaryDelegationBodyStreamParser<NAMESPACE_CAPACITY, TARGET_CAPACITY, ABI_CAPACITY>
{
    /// Creates an empty delegation body parser.
    pub fn new() -> Self {
        Self {
            queue: ByteQueue::new(),
            phase: Phase::Version,
            version: 0,
            expires: 0,
            developer_id: BoundedText::default(),
            key_id: KeyId([0; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
            namespaces: [BoundedText::default(); NAMESPACE_CAPACITY],
            namespace_count: 0,
            namespace_index: 0,
            targets: [BoundedText::default(); TARGET_CAPACITY],
            target_count: 0,
            target_index: 0,
            abis: [0; ABI_CAPACITY],
            abi_count: 0,
            abi_index: 0,
            text_length: 0,
            text: [0; TEXT_BYTES],
            not_before: 0,
            not_after: 0,
        }
    }

    /// Feeds one bounded transport fragment.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), StreamingBodyError> {
        for byte in bytes {
            self.queue.push(*byte).map_err(map_error)?;
            self.drive().map_err(map_error)?;
        }
        self.drive().map_err(map_error)
    }

    /// Completes parsing and validates the typed delegation contract.
    pub fn finish(&mut self) -> Result<DelegationMetadata, StreamingBodyError> {
        let mut output = MaybeUninit::uninit();
        self.finish_into(&mut output)?;
        // SAFETY: finish_into initializes output before returning Ok.
        Ok(unsafe { output.assume_init() })
    }

    /// Completes parsing directly into caller-owned output storage.
    pub fn finish_into(
        &mut self,
        output: &mut MaybeUninit<DelegationMetadata>,
    ) -> Result<(), StreamingBodyError> {
        self.drive().map_err(map_error)?;
        if self.phase != Phase::Complete || !self.queue.is_empty() {
            return Err(StreamingBodyError::UnexpectedEnd);
        }
        let mut namespaces = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
        namespaces[..usize::from(self.namespace_count)]
            .copy_from_slice(&self.namespaces[..usize::from(self.namespace_count)]);
        let mut targets = [BoundedText::default(); crate::MAX_DELEGATION_TARGETS];
        targets[..usize::from(self.target_count)]
            .copy_from_slice(&self.targets[..usize::from(self.target_count)]);
        let mut abis = [0; crate::MAX_DELEGATION_ABIS];
        abis[..usize::from(self.abi_count)]
            .copy_from_slice(&self.abis[..usize::from(self.abi_count)]);
        let metadata = DelegationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Delegation,
                version: self.version,
                expires: self.expires,
            },
            developer_id: self.developer_id,
            key_id: self.key_id,
            public_key: self.public_key,
            allowed_namespaces: namespaces,
            namespace_count: self.namespace_count,
            allowed_targets: targets,
            target_count: self.target_count,
            allowed_abis: abis,
            abi_count: self.abi_count,
            not_before: self.not_before,
            not_after: self.not_after,
        };
        validate_delegation(&metadata).map_err(|_| StreamingBodyError::InvalidBody)?;
        output.write(metadata);
        Ok(())
    }

    fn drive(&mut self) -> Result<(), Error> {
        loop {
            let progressed = match self.phase {
                Phase::Version => self.u64_field(|this, value| {
                    this.version = value;
                    this.phase = Phase::Expires
                }),
                Phase::Expires => self.u64_field(|this, value| {
                    this.expires = value;
                    this.phase = Phase::DeveloperLength
                }),
                Phase::DeveloperLength => self.text_length(),
                Phase::Developer => self.developer(),
                Phase::KeyId => self.fixed_key(),
                Phase::PublicKey => self.fixed_public(),
                Phase::NamespaceCount => self.count(0),
                Phase::NamespaceLength => self.text_length(),
                Phase::Namespace => self.namespace(),
                Phase::TargetCount => self.count(1),
                Phase::TargetLength => self.text_length(),
                Phase::Target => self.target(),
                Phase::AbiCount => self.count(2),
                Phase::Abi => self.abi(),
                Phase::NotBefore => self.u64_field(|this, value| {
                    this.not_before = value;
                    this.phase = Phase::NotAfter
                }),
                Phase::NotAfter => self.u64_field(|this, value| {
                    this.not_after = value;
                    this.phase = Phase::Complete
                }),
                Phase::Complete => Ok(false),
            }?;
            if !progressed {
                return Ok(());
            }
        }
    }

    fn u64_field(&mut self, set: impl FnOnce(&mut Self, u64)) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<8>() else {
            return Ok(false);
        };
        set(self, u64::from_le_bytes(value));
        Ok(true)
    }
    fn text_length(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        self.text_length = usize::from(u16::from_le_bytes(value));
        if self.text_length > TEXT_BYTES {
            return Err(Error::Invalid);
        }
        self.phase = match self.phase {
            Phase::DeveloperLength => Phase::Developer,
            Phase::NamespaceLength => Phase::Namespace,
            Phase::TargetLength => Phase::Target,
            _ => return Err(Error::Invalid),
        };
        Ok(true)
    }
    fn developer(&mut self) -> Result<bool, Error> {
        let Some(text) = self.text_value::<{ crate::MAX_DEVELOPER_ID_BYTES }>()? else {
            return Ok(false);
        };
        self.developer_id = text;
        self.phase = Phase::KeyId;
        Ok(true)
    }
    fn fixed_key(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<{ crate::KEY_ID_LENGTH }>() else {
            return Ok(false);
        };
        self.key_id = KeyId(value);
        self.phase = Phase::PublicKey;
        Ok(true)
    }
    fn fixed_public(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<{ crate::PUBLIC_KEY_LENGTH }>() else {
            return Ok(false);
        };
        self.public_key = PublicKey(value);
        self.phase = Phase::NamespaceCount;
        Ok(true)
    }
    fn count(&mut self, kind: u8) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        let limit = match kind {
            0 => NAMESPACE_CAPACITY,
            1 => TARGET_CAPACITY,
            _ => ABI_CAPACITY,
        };
        if usize::from(value[0]) > limit {
            return Err(Error::Invalid);
        }
        match kind {
            0 => {
                self.namespace_count = value[0];
                self.namespace_index = 0;
                self.phase = if value[0] == 0 {
                    Phase::TargetCount
                } else {
                    Phase::NamespaceLength
                };
            }
            1 => {
                self.target_count = value[0];
                self.target_index = 0;
                self.phase = if value[0] == 0 {
                    Phase::AbiCount
                } else {
                    Phase::TargetLength
                };
            }
            _ => {
                self.abi_count = value[0];
                self.abi_index = 0;
                self.phase = if value[0] == 0 {
                    Phase::NotBefore
                } else {
                    Phase::Abi
                };
            }
        }
        Ok(true)
    }
    fn namespace(&mut self) -> Result<bool, Error> {
        let Some(text) = self.text_value::<{ crate::MAX_NAMESPACE_BYTES }>()? else {
            return Ok(false);
        };
        self.namespaces[self.namespace_index] = text;
        self.namespace_index += 1;
        self.phase = if self.namespace_index == usize::from(self.namespace_count) {
            Phase::TargetCount
        } else {
            Phase::NamespaceLength
        };
        Ok(true)
    }
    fn target(&mut self) -> Result<bool, Error> {
        let Some(text) = self.text_value::<{ crate::MAX_TARGET_PROFILE_BYTES }>()? else {
            return Ok(false);
        };
        self.targets[self.target_index] = text;
        self.target_index += 1;
        self.phase = if self.target_index == usize::from(self.target_count) {
            Phase::AbiCount
        } else {
            Phase::TargetLength
        };
        Ok(true)
    }
    fn abi(&mut self) -> Result<bool, Error> {
        let Some(value) = self.queue.take::<2>() else {
            return Ok(false);
        };
        self.abis[self.abi_index] = u16::from_le_bytes(value);
        self.abi_index += 1;
        self.phase = if self.abi_index == usize::from(self.abi_count) {
            Phase::NotBefore
        } else {
            Phase::Abi
        };
        Ok(true)
    }
    fn text_value<const N: usize>(&mut self) -> Result<Option<BoundedText<N>>, Error> {
        if !self.queue.take_into(self.text_length, &mut self.text) {
            return Ok(None);
        }
        let text =
            core::str::from_utf8(&self.text[..self.text_length]).map_err(|_| Error::Invalid)?;
        BoundedText::new(text).map(Some).map_err(|_| Error::Invalid)
    }
}

impl<const NAMESPACE_CAPACITY: usize, const TARGET_CAPACITY: usize, const ABI_CAPACITY: usize>
    Default
    for BinaryDelegationBodyStreamParser<NAMESPACE_CAPACITY, TARGET_CAPACITY, ABI_CAPACITY>
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Error {
    QueueFull,
    Invalid,
}
fn map_error(error: Error) -> StreamingBodyError {
    match error {
        Error::QueueFull => StreamingBodyError::BodyTooLarge,
        Error::Invalid => StreamingBodyError::InvalidBody,
    }
}

struct ByteQueue {
    bytes: [u8; QUEUE_BYTES],
    length: usize,
}
impl ByteQueue {
    const fn new() -> Self {
        Self {
            bytes: [0; QUEUE_BYTES],
            length: 0,
        }
    }
    fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.length == QUEUE_BYTES {
            return Err(Error::QueueFull);
        }
        self.bytes[self.length] = byte;
        self.length += 1;
        Ok(())
    }
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        if self.length < N {
            return None;
        }
        let mut out = [0; N];
        out.copy_from_slice(&self.bytes[..N]);
        self.bytes.copy_within(N..self.length, 0);
        self.length -= N;
        Some(out)
    }
    fn take_into(&mut self, length: usize, out: &mut [u8]) -> bool {
        if length > out.len() || self.length < length {
            return false;
        }
        out[..length].copy_from_slice(&self.bytes[..length]);
        self.bytes.copy_within(length..self.length, 0);
        self.length -= length;
        true
    }
    const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_fragmented_delegation_body() {
        let developer = |value| BoundedText::new(value).expect("developer text fits");
        let namespace = |value| BoundedText::new(value).expect("namespace text fits");
        let target = |value| BoundedText::new(value).expect("target text fits");
        let metadata = DelegationMetadata {
            header: MetadataHeader {
                role: MetadataRole::Delegation,
                version: 1,
                expires: 0,
            },
            developer_id: developer("developer"),
            key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
            allowed_namespaces: [namespace("apps"); crate::MAX_DELEGATION_SCOPES],
            namespace_count: 1,
            allowed_targets: [target("f405"); crate::MAX_DELEGATION_TARGETS],
            target_count: 1,
            allowed_abis: [3; crate::MAX_DELEGATION_ABIS],
            abi_count: 1,
            not_before: 0,
            not_after: 0,
        };
        let mut body = [0; crate::MAX_DELEGATION_BYTES];
        let length =
            crate::encode_binary_delegation_body(metadata, &mut body).expect("delegation encodes");
        let mut parser = BinaryDelegationBodyStreamParser::<
            { crate::MAX_DELEGATION_SCOPES },
            { crate::MAX_DELEGATION_TARGETS },
            { crate::MAX_DELEGATION_ABIS },
        >::new();
        for chunk in body[..length].chunks(7) {
            parser.feed(chunk).expect("fragment parses")
        }
        let parsed = parser.finish().expect("delegation finishes");
        assert_eq!(parsed.developer_id, metadata.developer_id);
        assert_eq!(parsed.key_id, metadata.key_id);
        assert_eq!(parsed.public_key, metadata.public_key);
        assert_eq!(parsed.namespace_count, 1);
        assert_eq!(parsed.allowed_namespaces[0], metadata.allowed_namespaces[0]);
        assert_eq!(parsed.target_count, 1);
        assert_eq!(parsed.allowed_targets[0], metadata.allowed_targets[0]);
        assert_eq!(parsed.abi_count, 1);
        assert_eq!(parsed.allowed_abis[0], 3);
    }
}
