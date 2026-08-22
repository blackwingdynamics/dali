//! Validation for the package-free durable trust-store payload.

use super::{MetadataError, validate_target_profile};

/// Validates the package-free payload stored in a durable trust-store slot.
pub fn validate_trust_store_payload(
    metadata: &crate::TrustStorePayload,
) -> Result<(), MetadataError> {
    if metadata.header.role != crate::MetadataRole::Recovery
        || metadata.header.version == 0
        || usize::from(metadata.file_count) > crate::MAX_TRUST_STORE_FILES
    {
        return Err(MetadataError::InvalidBundle);
    }
    let target_profile = metadata
        .target_profile
        .as_str()
        .ok_or(MetadataError::InvalidBundle)?;
    validate_target_profile(target_profile).map_err(|_| MetadataError::InvalidBundle)?;
    let files = &metadata.files[..usize::from(metadata.file_count)];
    if files.is_empty() || !has_required_roles(files) {
        return Err(MetadataError::InvalidBundle);
    }
    for (index, file) in files.iter().enumerate() {
        let id = file.id.as_str().ok_or(MetadataError::InvalidBundle)?;
        if file.kind == crate::BundleFileKind::Package
            || file.length == 0
            || file.sha256.0 == [0; crate::SHA256_LENGTH]
            || files[..index]
                .iter()
                .any(|candidate| (candidate.kind, candidate.id) == (file.kind, file.id))
            || (index > 0 && file_order(files[index - 1], *file) != core::cmp::Ordering::Less)
        {
            return Err(MetadataError::InvalidBundle);
        }
        if is_singleton(file.kind) && id != file.kind.as_str() {
            return Err(MetadataError::InvalidBundle);
        }
    }
    Ok(())
}

fn has_required_roles(files: &[crate::BundleFile]) -> bool {
    [
        crate::BundleFileKind::Root,
        crate::BundleFileKind::Timestamp,
        crate::BundleFileKind::Snapshot,
        crate::BundleFileKind::Targets,
        crate::BundleFileKind::Revocation,
        crate::BundleFileKind::Delegation,
    ]
    .iter()
    .all(|kind| files.iter().any(|file| file.kind == *kind))
}

fn is_singleton(kind: crate::BundleFileKind) -> bool {
    !matches!(kind, crate::BundleFileKind::Delegation)
}

fn file_order(left: crate::BundleFile, right: crate::BundleFile) -> core::cmp::Ordering {
    kind_order(left.kind)
        .cmp(&kind_order(right.kind))
        .then_with(|| {
            left.id
                .as_str()
                .unwrap_or("")
                .cmp(right.id.as_str().unwrap_or(""))
        })
}

fn kind_order(kind: crate::BundleFileKind) -> u8 {
    match kind {
        crate::BundleFileKind::Root => 0,
        crate::BundleFileKind::Timestamp => 1,
        crate::BundleFileKind::Snapshot => 2,
        crate::BundleFileKind::Targets => 3,
        crate::BundleFileKind::Revocation => 4,
        crate::BundleFileKind::Delegation => 5,
        crate::BundleFileKind::Package => 6,
    }
}
