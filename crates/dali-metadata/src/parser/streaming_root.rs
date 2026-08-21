//! Incremental Binary Metadata v2 root-body parser.

use crate::{
    KeyId, MetadataHeader, MetadataRole, PublicKey, RoleDefinition, RoleKey, RootMetadata,
    StreamingBodyError, validate_role_references,
};

const QUEUE_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Version,
    Expires,
    KeyCount,
    KeyId,
    KeyRole,
    KeyPublic,
    RoleCount,
    RoleRole,
    RoleThreshold,
    RoleKeyCount,
    RoleKeyId,
    Complete,
}

/// Incrementally parses a Binary v2 root body without retaining the body.
pub struct BinaryRootBodyStreamParser {
    queue: ByteQueue,
    phase: Phase,
    version: u64,
    expires: u64,
    keys: [RoleKey; crate::MAX_ROOT_KEYS],
    key_count: u8,
    key_index: usize,
    roles: [RoleDefinition; crate::MAX_ROOT_ROLES],
    role_count: u8,
    role_index: usize,
    role_key_index: usize,
}

impl BinaryRootBodyStreamParser {
    /// Creates an empty root body parser.
    pub fn new() -> Self {
        let key = RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([0; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
        };
        let role = RoleDefinition {
            role: MetadataRole::Root,
            keys: [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
            key_count: 0,
            threshold: 0,
        };
        Self {
            queue: ByteQueue::new(),
            phase: Phase::Version,
            version: 0,
            expires: 0,
            keys: [key; crate::MAX_ROOT_KEYS],
            key_count: 0,
            key_index: 0,
            roles: [role; crate::MAX_ROOT_ROLES],
            role_count: 0,
            role_index: 0,
            role_key_index: 0,
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

    /// Completes parsing and validates key references.
    pub fn finish(mut self) -> Result<RootMetadata, StreamingBodyError> {
        self.drive().map_err(map_error)?;
        if self.phase != Phase::Complete || !self.queue.is_empty() {
            return Err(StreamingBodyError::UnexpectedEnd);
        }
        validate_role_references(
            &self.keys[..usize::from(self.key_count)],
            &self.roles[..usize::from(self.role_count)],
        )
        .map_err(|_| StreamingBodyError::InvalidBody)?;
        Ok(RootMetadata {
            header: MetadataHeader {
                role: MetadataRole::Root,
                version: self.version,
                expires: self.expires,
            },
            keys: self.keys,
            key_count: self.key_count,
            roles: self.roles,
            role_count: self.role_count,
        })
    }

    fn drive(&mut self) -> Result<(), RootError> {
        loop {
            let progressed = match self.phase {
                Phase::Version => self.u64_field(|this, value| {
                    this.version = value;
                    this.phase = Phase::Expires
                }),
                Phase::Expires => self.u64_field(|this, value| {
                    this.expires = value;
                    this.phase = Phase::KeyCount
                }),
                Phase::KeyCount => self.count(true),
                Phase::KeyId => self.key_id(),
                Phase::KeyRole => self.role(),
                Phase::KeyPublic => self.public_key(),
                Phase::RoleCount => self.count(false),
                Phase::RoleRole => self.role_definition(),
                Phase::RoleThreshold => self.role_threshold(),
                Phase::RoleKeyCount => self.role_key_count(),
                Phase::RoleKeyId => self.role_key_id(),
                Phase::Complete => Ok(false),
            }?;
            if !progressed {
                return Ok(());
            }
        }
    }

    fn u64_field(&mut self, set: impl FnOnce(&mut Self, u64)) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<8>() else {
            return Ok(false);
        };
        set(self, u64::from_le_bytes(value));
        Ok(true)
    }

    fn count(&mut self, keys: bool) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        if value[0] == 0
            || (keys && usize::from(value[0]) > crate::MAX_ROOT_KEYS)
            || (!keys && usize::from(value[0]) > crate::MAX_ROOT_ROLES)
        {
            return Err(RootError::Invalid);
        }
        if keys {
            self.key_count = value[0];
            self.key_index = 0;
            self.phase = Phase::KeyId;
        } else {
            self.role_count = value[0];
            self.role_index = 0;
            self.phase = Phase::RoleRole;
        }
        Ok(true)
    }

    fn key_id(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<{ crate::KEY_ID_LENGTH }>() else {
            return Ok(false);
        };
        self.keys[self.key_index].key_id = KeyId(value);
        self.phase = Phase::KeyRole;
        Ok(true)
    }

    fn role(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        self.keys[self.key_index].role =
            crate::codec::binary::role_from_number(value[0]).ok_or(RootError::Invalid)?;
        self.phase = Phase::KeyPublic;
        Ok(true)
    }

    fn public_key(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<{ crate::PUBLIC_KEY_LENGTH }>() else {
            return Ok(false);
        };
        self.keys[self.key_index].public_key = PublicKey(value);
        self.key_index += 1;
        self.phase = if self.key_index == usize::from(self.key_count) {
            Phase::RoleCount
        } else {
            Phase::KeyId
        };
        Ok(true)
    }

    fn role_definition(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        self.roles[self.role_index].role =
            crate::codec::binary::role_from_number(value[0]).ok_or(RootError::Invalid)?;
        self.phase = Phase::RoleThreshold;
        Ok(true)
    }

    fn role_threshold(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        self.roles[self.role_index].threshold = value[0];
        self.phase = Phase::RoleKeyCount;
        Ok(true)
    }

    fn role_key_count(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<1>() else {
            return Ok(false);
        };
        if value[0] == 0 || usize::from(value[0]) > crate::MAX_ROLE_KEYS {
            return Err(RootError::Invalid);
        }
        self.roles[self.role_index].key_count = value[0];
        self.role_key_index = 0;
        self.phase = Phase::RoleKeyId;
        Ok(true)
    }

    fn role_key_id(&mut self) -> Result<bool, RootError> {
        let Some(value) = self.queue.take::<{ crate::KEY_ID_LENGTH }>() else {
            return Ok(false);
        };
        self.roles[self.role_index].keys[self.role_key_index] = KeyId(value);
        self.role_key_index += 1;
        if self.role_key_index == usize::from(self.roles[self.role_index].key_count) {
            self.role_index += 1;
            self.phase = if self.role_index == usize::from(self.role_count) {
                Phase::Complete
            } else {
                Phase::RoleRole
            };
        }
        Ok(true)
    }
}

impl Default for BinaryRootBodyStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootError {
    QueueFull,
    Invalid,
}

fn map_error(error: RootError) -> StreamingBodyError {
    match error {
        RootError::QueueFull => StreamingBodyError::BodyTooLarge,
        RootError::Invalid => StreamingBodyError::InvalidBody,
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
    fn push(&mut self, byte: u8) -> Result<(), RootError> {
        if self.length == QUEUE_BYTES {
            return Err(RootError::QueueFull);
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
    const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fragmented_root_body() {
        let key = RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([1; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
        };
        let role = RoleDefinition {
            role: MetadataRole::Root,
            keys: [key.key_id; crate::MAX_ROLE_KEYS],
            key_count: 1,
            threshold: 1,
        };
        let metadata = RootMetadata {
            header: MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            keys: [key; crate::MAX_ROOT_KEYS],
            key_count: 1,
            roles: [role; crate::MAX_ROOT_ROLES],
            role_count: 1,
        };
        let mut body = [0; crate::MAX_ROOT_BYTES];
        let length = crate::encode_binary_root_body(metadata, &mut body).expect("root encodes");
        let mut parser = BinaryRootBodyStreamParser::new();
        for chunk in body[..length].chunks(5) {
            parser.feed(chunk).expect("fragment parses");
        }
        let parsed = parser.finish().expect("root finishes");
        assert_eq!(parsed.header, metadata.header);
        assert_eq!(parsed.key_count, metadata.key_count);
        assert_eq!(parsed.keys[0], metadata.keys[0]);
        assert_eq!(parsed.role_count, metadata.role_count);
        assert_eq!(parsed.roles[0].role, metadata.roles[0].role);
        assert_eq!(parsed.roles[0].threshold, metadata.roles[0].threshold);
        assert_eq!(parsed.roles[0].key_count, metadata.roles[0].key_count);
        assert_eq!(parsed.roles[0].keys[0], metadata.roles[0].keys[0]);
    }
}
