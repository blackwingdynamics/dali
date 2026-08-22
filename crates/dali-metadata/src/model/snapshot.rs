use crate::{BoundedText, MAX_DELEGATION_ID_BYTES, MetadataHeader, Sha256Digest};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetsReference {
    pub version: u64,
    pub length: u32,
    pub sha256: Sha256Digest,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DelegationReference {
    pub id: BoundedText<MAX_DELEGATION_ID_BYTES>,
    pub version: u64,
    pub length: u32,
    pub sha256: Sha256Digest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    pub header: MetadataHeader,
    pub targets: TargetsReference,
    pub revocations: crate::RevocationReference,
    pub delegations: [DelegationReference; crate::MAX_SNAPSHOT_REFERENCES],
    pub delegation_count: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampMetadata {
    pub header: MetadataHeader,
    pub snapshot_version: u64,
    pub snapshot_length: u32,
    pub snapshot_sha256: Sha256Digest,
}
