use super::{MetadataError, validate_target_profile};

pub fn validate_bundle_metadata(metadata: &crate::BundleMetadata) -> Result<(), MetadataError> {
    if metadata.header.role != crate::MetadataRole::Bundle
        || metadata.header.version == 0
        || usize::from(metadata.file_count) > crate::MAX_BUNDLE_FILES
    {
        return Err(MetadataError::InvalidBundle);
    }
    validate_target_profile(
        metadata
            .target_profile
            .as_str()
            .ok_or(MetadataError::InvalidBundle)?,
    )
    .map_err(|_| MetadataError::InvalidBundle)?;
    let files = &metadata.files[..usize::from(metadata.file_count)];
    if files.is_empty() || !required_kinds_present(files) {
        return Err(MetadataError::InvalidBundle);
    }
    for (index, file) in files.iter().enumerate() {
        let id = file.id.as_str().ok_or(MetadataError::InvalidBundle)?;
        if file.length == 0
            || file.sha256.0 == [0; crate::SHA256_LENGTH]
            || files[..index]
                .iter()
                .any(|candidate| (candidate.kind, candidate.id) == (file.kind, file.id))
            || (index > 0 && file_order(files[index - 1], *file) != core::cmp::Ordering::Less)
        {
            return Err(MetadataError::InvalidBundle);
        }
        if matches!(
            file.kind,
            crate::BundleFileKind::Root
                | crate::BundleFileKind::Timestamp
                | crate::BundleFileKind::Snapshot
                | crate::BundleFileKind::Targets
                | crate::BundleFileKind::Revocation
        ) && id != file.kind.as_str()
        {
            return Err(MetadataError::InvalidBundle);
        }
    }
    Ok(())
}

fn required_kinds_present(files: &[crate::BundleFile]) -> bool {
    [
        crate::BundleFileKind::Root,
        crate::BundleFileKind::Timestamp,
        crate::BundleFileKind::Snapshot,
        crate::BundleFileKind::Targets,
        crate::BundleFileKind::Revocation,
        crate::BundleFileKind::Delegation,
        crate::BundleFileKind::Cartridge,
    ]
    .iter()
    .all(|kind| files.iter().any(|file| file.kind == *kind))
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
        crate::BundleFileKind::Cartridge => 6,
    }
}
