use crate::{KEY_ID_LENGTH, PUBLIC_KEY_LENGTH, SHA256_LENGTH, SIGNATURE_LENGTH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataRole {
    Root,
    Timestamp,
    Snapshot,
    Targets,
    Delegation,
    Revocation,
    Recovery,
    Bundle,
}

impl MetadataRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Timestamp => "timestamp",
            Self::Snapshot => "snapshot",
            Self::Targets => "targets",
            Self::Delegation => "delegation",
            Self::Revocation => "revocation",
            Self::Recovery => "recovery",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyId(pub [u8; KEY_ID_LENGTH]);
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CartridgeId(pub [u8; KEY_ID_LENGTH]);
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sha256Digest(pub [u8; SHA256_LENGTH]);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    Empty,
    TooLong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedText<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    length: u16,
}
impl<const CAPACITY: usize> Default for BoundedText<CAPACITY> {
    fn default() -> Self {
        Self {
            bytes: [0; CAPACITY],
            length: 0,
        }
    }
}
impl<const CAPACITY: usize> BoundedText<CAPACITY> {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; CAPACITY],
            length: 0,
        }
    }

    pub fn new(value: &str) -> Result<Self, TextError> {
        if value.is_empty() {
            return Err(TextError::Empty);
        }
        if value.len() > CAPACITY || value.len() > usize::from(u16::MAX) {
            return Err(TextError::TooLong);
        }
        let mut bytes = [0; CAPACITY];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            length: value.len() as u16,
        })
    }
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.bytes[..usize::from(self.length)]).ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicKey(pub [u8; PUBLIC_KEY_LENGTH]);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature(pub [u8; SIGNATURE_LENGTH]);
impl Default for Signature {
    fn default() -> Self {
        Self([0; SIGNATURE_LENGTH])
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SignatureRecord {
    pub key_id: KeyId,
    pub signature: Signature,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureSet {
    pub records: [SignatureRecord; crate::MAX_SIGNATURES],
    pub count: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignedEnvelope<'a> {
    pub signed: &'a [u8],
    pub signatures: SignatureSet,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataHeader {
    pub role: MetadataRole,
    pub version: u64,
    pub expires: u64,
}
