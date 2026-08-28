/// Internal helper for `role`.
fn role<E>(
    root: &RootMetadata,
    expected_role: MetadataRole,
) -> Result<RoleDefinition, BinaryRepositoryError<E>> {
    root.roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|definition| definition.role == expected_role)
        .copied()
        .ok_or(BinaryRepositoryError::MissingRecord)
}

/// Internal helper for `map_root_error`.
fn map_root_error<E>(error: StreamedRoleError<E>) -> BinaryRepositoryError<E> {
    match error {
        StreamedRoleError::UnknownTrustAnchor => BinaryRepositoryError::UnknownTrustAnchor,
        other => map_role_error(other),
    }
}

/// Internal helper for `map_role_error`.
fn map_role_error<E>(error: StreamedRoleError<E>) -> BinaryRepositoryError<E> {
    match error {
        StreamedRoleError::Storage(error) => BinaryRepositoryError::RoleStorage(error),
        StreamedRoleError::Signature => BinaryRepositoryError::RoleSignature,
        StreamedRoleError::Decode | StreamedRoleError::LengthMismatch => {
            BinaryRepositoryError::RoleDecode
        }
        StreamedRoleError::UnknownTrustAnchor => BinaryRepositoryError::UnknownTrustAnchor,
    }
}

/// Internal helper for `same_reference`.
fn same_reference(
    expected_version: u64,
    expected_length: u32,
    expected_digest: Sha256Digest,
    actual_version: u64,
    actual_length: u32,
    actual_digest: Sha256Digest,
) -> bool {
    expected_version == actual_version
        && expected_length == actual_length
        && expected_digest == actual_digest
}

/// Internal helper for `validate_target_delegation`.
fn validate_target_delegation<E>(
    target: dali_metadata::TargetCartridge,
    delegation: &dali_metadata::DelegationMetadata,
) -> Result<(), BinaryRepositoryError<E>> {
    let namespace_allowed = delegation.allowed_namespaces
        [..usize::from(delegation.namespace_count)]
        .contains(&target.namespace);
    let target_allowed = delegation.allowed_targets[..usize::from(delegation.target_count)]
        .contains(&target.target_profile);
    let abi_allowed =
        delegation.allowed_abis[..usize::from(delegation.abi_count)].contains(&target.abi_version);
    if target.developer_id != delegation.developer_id
        || target.developer_key_id != delegation.key_id
        || !namespace_allowed
        || !target_allowed
        || !abi_allowed
    {
        return Err(BinaryRepositoryError::DelegationMismatch);
    }
    Ok(())
}
