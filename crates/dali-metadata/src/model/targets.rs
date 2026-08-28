use crate::{
    BoundedText, CartridgeId, KeyId, MAX_CARTRIDGE_VERSION_BYTES, MAX_DELEGATION_ID_BYTES,
    MAX_DELEGATION_SCOPES, MAX_DEVELOPER_ID_BYTES, MAX_NAMESPACE_BYTES, MAX_TARGET_PROFILE_BYTES,
    MetadataHeader, Sha256Digest,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetCartridge {
    pub cartridge_id: CartridgeId,
    pub namespace: BoundedText<MAX_NAMESPACE_BYTES>,
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    pub delegation_id: BoundedText<MAX_DELEGATION_ID_BYTES>,
    pub developer_key_id: KeyId,
    pub target_profile: BoundedText<MAX_TARGET_PROFILE_BYTES>,
    pub amrn_format: u16,
    pub abi_version: u16,
    pub cartridge_version: BoundedText<MAX_CARTRIDGE_VERSION_BYTES>,
    pub minimum_kernel_version: BoundedText<MAX_CARTRIDGE_VERSION_BYTES>,
    pub length: u32,
    pub sha256: Sha256Digest,
    pub required_services: u32,
    pub slot_id: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetsMetadata {
    pub header: MetadataHeader,
    pub delegations: [BoundedText<MAX_DELEGATION_ID_BYTES>; MAX_DELEGATION_SCOPES],
    pub delegation_count: u8,
    pub cartridges: [TargetCartridge; crate::MAX_TARGET_RECORDS],
    pub cartridge_count: u16,
}
