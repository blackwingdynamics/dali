//! Provisioned trust-anchor checks for repository root metadata.

use dali_metadata::{MetadataRole, RootMetadata};
use dali_targets::TrustAnchorProfile;

/// Confirms that a target-provisioned anchor is declared as a root key.
pub(crate) fn contains_root_anchor(root: &RootMetadata, anchor: TrustAnchorProfile) -> bool {
    root.keys
        .iter()
        .take(usize::from(root.key_count))
        .any(|key| {
            key.role == MetadataRole::Root
                && key.key_id.0 == anchor.key_id
                && key.public_key.0 == anchor.public_key
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dali_metadata::{KeyId, MetadataHeader, PublicKey, RoleKey};

    #[test]
    fn accepts_only_the_provisioned_root_key() {
        let key = RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([1; dali_metadata::KEY_ID_LENGTH]),
            public_key: PublicKey([2; dali_metadata::PUBLIC_KEY_LENGTH]),
        };
        let root = RootMetadata {
            header: MetadataHeader {
                role: MetadataRole::Root,
                version: 1,
                expires: 0,
            },
            keys: [key; dali_metadata::MAX_ROOT_KEYS],
            key_count: 1,
            roles: [dali_metadata::RoleDefinition {
                role: MetadataRole::Root,
                keys: [key.key_id; dali_metadata::MAX_ROLE_KEYS],
                key_count: 1,
                threshold: 1,
            }; dali_metadata::MAX_ROOT_ROLES],
            role_count: 1,
        };
        let anchor = TrustAnchorProfile {
            key_id: key.key_id.0,
            public_key: key.public_key.0,
        };
        assert!(contains_root_anchor(&root, anchor));
    }
}
