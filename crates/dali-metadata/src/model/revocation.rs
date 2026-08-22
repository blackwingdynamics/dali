use crate::{BoundedText, KeyId, MAX_DEVELOPER_ID_BYTES, MetadataHeader, Sha256Digest};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevocationRecord {
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    pub effective_version: u64,
    pub issuer_key_id: KeyId,
    pub key_id: KeyId,
    pub reason: BoundedText<{ crate::MAX_REVOCATION_REASON_BYTES }>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevocationMetadata {
    pub header: MetadataHeader,
    pub records: [RevocationRecord; crate::MAX_REVOCATIONS],
    pub record_count: u8,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevocationReference {
    pub version: u64,
    pub length: u32,
    pub sha256: Sha256Digest,
}
