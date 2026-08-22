use crate::{
    BoundedText, KeyId, MAX_DELEGATION_ABIS, MAX_DELEGATION_SCOPES, MAX_DELEGATION_TARGETS,
    MAX_DEVELOPER_ID_BYTES, MAX_TARGET_PROFILE_BYTES, MetadataHeader, PublicKey,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationMetadata {
    pub header: MetadataHeader,
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    pub key_id: KeyId,
    pub public_key: PublicKey,
    pub allowed_namespaces: [BoundedText<{ crate::MAX_NAMESPACE_BYTES }>; MAX_DELEGATION_SCOPES],
    pub namespace_count: u8,
    pub allowed_targets: [BoundedText<MAX_TARGET_PROFILE_BYTES>; MAX_DELEGATION_TARGETS],
    pub target_count: u8,
    pub allowed_abis: [u16; MAX_DELEGATION_ABIS],
    pub abi_count: u8,
    pub not_before: u64,
    pub not_after: u64,
}
