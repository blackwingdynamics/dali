use crate::{
    BoundedText, KeyId, MAX_DELEGATION_ID_BYTES, MAX_DELEGATION_SCOPES, MAX_DEVELOPER_ID_BYTES,
    MAX_NAMESPACE_BYTES, MAX_PACKAGE_VERSION_BYTES, MAX_TARGET_PROFILE_BYTES, MetadataHeader,
    PackageId, Sha256Digest,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetPackage {
    pub package_id: PackageId,
    pub namespace: BoundedText<MAX_NAMESPACE_BYTES>,
    pub developer_id: BoundedText<MAX_DEVELOPER_ID_BYTES>,
    pub delegation_id: BoundedText<MAX_DELEGATION_ID_BYTES>,
    pub developer_key_id: KeyId,
    pub target_profile: BoundedText<MAX_TARGET_PROFILE_BYTES>,
    pub amrn_format: u16,
    pub abi_version: u16,
    pub package_version: BoundedText<MAX_PACKAGE_VERSION_BYTES>,
    pub minimum_kernel_version: BoundedText<MAX_PACKAGE_VERSION_BYTES>,
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
    pub packages: [TargetPackage; crate::MAX_TARGET_RECORDS],
    pub package_count: u16,
}
