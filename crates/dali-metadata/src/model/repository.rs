use super::{KeyId, MetadataHeader, MetadataRole, PublicKey};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleKey {
    pub role: MetadataRole,
    pub key_id: KeyId,
    pub public_key: PublicKey,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleDefinition {
    pub role: MetadataRole,
    pub keys: [KeyId; crate::MAX_ROLE_KEYS],
    pub key_count: u8,
    pub threshold: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootMetadata {
    pub header: MetadataHeader,
    pub keys: [RoleKey; crate::MAX_ROOT_KEYS],
    pub key_count: u8,
    pub roles: [RoleDefinition; crate::MAX_ROOT_ROLES],
    pub role_count: u8,
}
