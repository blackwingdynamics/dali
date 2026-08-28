#[inline(never)]
/// Internal helper for `verify_timestamp_and_snapshot`.
fn verify_timestamp_and_snapshot<S>(
    storage: &mut S,
    root: &RootMetadata,
    chunk: &mut [u8],
    output: &mut MaybeUninit<dali_metadata::SnapshotMetadata>,
    role_verifier: &mut MaybeUninit<StreamingRoleVerifier>,
    progress: fn() -> bool,
) -> Result<(), BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut timestamp_output = MaybeUninit::<dali_metadata::TimestampMetadata>::uninit();
    let mut timestamp_parser = BinaryTimestampBodyStreamParser::new();
    let _timestamp_info = verify_role_from_root(
        storage,
        RepositoryDocument::Timestamp,
        MetadataRole::Timestamp,
        &mut timestamp_parser,
        root,
        RoleVerificationContext {
            output: &mut timestamp_output,
            role_verifier,
            input: RoleVerificationInput { chunk, progress },
        },
    )?;
    // SAFETY: verify_role_from_root writes the timestamp before returning.
    let timestamp_metadata = unsafe { timestamp_output.assume_init_ref() };
    let mut snapshot_parser =
        BinarySnapshotBodyStreamParser::<MAX_BINARY_REPOSITORY_CARTRIDGES>::new();
    let snapshot = verify_role_from_root(
        storage,
        RepositoryDocument::Snapshot,
        MetadataRole::Snapshot,
        &mut snapshot_parser,
        root,
        RoleVerificationContext {
            output,
            role_verifier,
            input: RoleVerificationInput { chunk, progress },
        },
    )?;
    // SAFETY: verify_role_from_root writes the snapshot before returning.
    let snapshot_metadata = unsafe { output.assume_init_ref() };
    if !same_reference(
        timestamp_metadata.snapshot_version,
        timestamp_metadata.snapshot_length,
        timestamp_metadata.snapshot_sha256,
        snapshot_metadata.header.version,
        snapshot.length,
        snapshot.digest,
    ) {
        return Err(BinaryRepositoryError::SnapshotReferenceMismatch);
    }
    Ok(())
}

#[inline(never)]
/// Internal helper for `verify_revocations`.
fn verify_revocations<S>(
    storage: &mut S,
    root: &RootMetadata,
    snapshot: &dali_metadata::SnapshotMetadata,
    chunk: &mut [u8],
    output: &mut MaybeUninit<dali_metadata::RevocationMetadata>,
    role_verifier: &mut MaybeUninit<StreamingRoleVerifier>,
    progress: fn() -> bool,
) -> Result<(), BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let mut parser = BinaryRevocationBodyStreamParser::<MAX_BINARY_REVOCATION_RECORDS>::new();
    let revocations = verify_role_from_root(
        storage,
        RepositoryDocument::Revocations,
        MetadataRole::Revocation,
        &mut parser,
        root,
        RoleVerificationContext {
            output,
            role_verifier,
            input: RoleVerificationInput { chunk, progress },
        },
    )?;
    // SAFETY: verify_role_from_root writes the revocations before returning.
    let revocation_metadata = unsafe { output.assume_init_ref() };
    if !same_reference(
        snapshot.revocations.version,
        snapshot.revocations.length,
        snapshot.revocations.sha256,
        revocation_metadata.header.version,
        revocations.length,
        revocations.digest,
    ) {
        return Err(BinaryRepositoryError::RevocationReferenceMismatch);
    }
    Ok(())
}

#[inline(never)]
/// Internal helper for `verify_delegation_and_cartridge`.
fn verify_delegation_and_cartridge<S>(
    context: &mut CartridgeVerificationContext<'_, S>,
    target: dali_metadata::TargetCartridge,
    target_version: u64,
    contract: dali_amrn::v3::Contract,
) -> Result<[u8; dali_metadata::PUBLIC_KEY_LENGTH], BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let delegation_id = target
        .delegation_id
        .as_str()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    let mut parser = BinaryDelegationBodyStreamParser::<
        MAX_BINARY_CARTRIDGE_DELEGATION_NAMESPACES,
        MAX_BINARY_CARTRIDGE_DELEGATION_TARGETS,
        MAX_BINARY_CARTRIDGE_DELEGATION_ABIS,
    >::new();
    let delegation_info = verify_role_from_root(
        context.storage,
        RepositoryDocument::Delegation(delegation_id),
        MetadataRole::Delegation,
        &mut parser,
        context.root,
        RoleVerificationContext {
            output: context.delegation_output,
            role_verifier: context.role_verifier,
            input: RoleVerificationInput {
                chunk: context.chunk,
                progress: context.progress,
            },
        },
    )?;
    // SAFETY: verify_role_from_root writes the delegation before returning.
    let delegation = unsafe { context.delegation_output.assume_init_ref() };
    if !same_reference(
        context.delegation_reference.version,
        context.delegation_reference.length,
        context.delegation_reference.sha256,
        delegation.header.version,
        delegation_info.length,
        delegation_info.digest,
    ) {
        return Err(BinaryRepositoryError::DelegationReferenceMismatch);
    }
    validate_target_delegation(target, delegation)?;
    if is_revoked(context.revocations, delegation, target_version) {
        return Err(BinaryRepositoryError::Revoked);
    }
    amrn::verify_streamed_amrn(
        context.storage,
        RepositoryCartridgeDigest(target.sha256.0),
        delegation,
        contract,
        context.chunk,
        context.amrn_buffers,
    )
    .map(|_| delegation.public_key.0)
    .map_err(map_amrn_error)
}

/// Internal helper for `map_amrn_error`.
fn map_amrn_error<E>(error: amrn::AmrnStreamError<E>) -> BinaryRepositoryError<E> {
    match error {
        amrn::AmrnStreamError::Storage(error) => BinaryRepositoryError::CartridgeStorage(error),
        amrn::AmrnStreamError::LengthMismatch => BinaryRepositoryError::CartridgeLengthMismatch,
        amrn::AmrnStreamError::InvalidHeader(error) => {
            BinaryRepositoryError::CartridgeInvalidHeader(error)
        }
        amrn::AmrnStreamError::DigestMismatch => BinaryRepositoryError::CartridgeDigestMismatch,
        amrn::AmrnStreamError::InvalidSignature => BinaryRepositoryError::CartridgeSignature,
        amrn::AmrnStreamError::InvalidCrc => BinaryRepositoryError::CartridgeCrc,
    }
}

/// Internal helper for `map_targets_error`.
fn map_targets_error<E>(
    error: streaming::StreamingTargetSelectionError<E>,
) -> BinaryRepositoryError<E> {
    match error {
        streaming::StreamingTargetSelectionError::Storage(error) => {
            BinaryRepositoryError::RoleStorage(error)
        }
        streaming::StreamingTargetSelectionError::Parse(error) => {
            BinaryRepositoryError::TargetsParse(error)
        }
        streaming::StreamingTargetSelectionError::Verification => {
            BinaryRepositoryError::RoleSignature
        }
    }
}

/// Internal implementation state for `CartridgeVerificationContext`.
struct CartridgeVerificationContext<'a, S> {
/// Internal field `storage`.
    storage: &'a mut S,
/// Internal field `root`.
    root: &'a RootMetadata,
/// Internal field `revocations`.
    revocations: &'a dali_metadata::RevocationMetadata,
/// Internal field `chunk`.
    chunk: &'a mut [u8],
/// Internal field `amrn_buffers`.
    amrn_buffers: &'a mut amrn::AmrnStreamBuffers,
/// Internal field `delegation_reference`.
    delegation_reference: dali_metadata::DelegationReference,
/// Internal field `delegation_output`.
    delegation_output: &'a mut MaybeUninit<dali_metadata::DelegationMetadata>,
/// Internal field `role_verifier`.
    role_verifier: &'a mut MaybeUninit<StreamingRoleVerifier>,
/// Internal field `progress`.
    progress: fn() -> bool,
}

/// Internal implementation state for `RoleVerificationInput`.
struct RoleVerificationInput<'a> {
/// Internal field `chunk`.
    chunk: &'a mut [u8],
/// Internal field `progress`.
    progress: fn() -> bool,
}

/// Internal implementation state for `RoleVerificationContext`.
struct RoleVerificationContext<'a, T> {
/// Internal field `output`.
    output: &'a mut MaybeUninit<T>,
/// Internal field `role_verifier`.
    role_verifier: &'a mut MaybeUninit<StreamingRoleVerifier>,
/// Internal field `input`.
    input: RoleVerificationInput<'a>,
}

/// Internal implementation state for `RoleVerificationPolicy`.
struct RoleVerificationPolicy<'a> {
/// Internal field `role`.
    role: RoleDefinition,
/// Internal field `keys`.
    keys: &'a [RoleKey],
}

#[inline(never)]
/// Internal helper for `verify_role_from_root`.
fn verify_role_from_root<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    parser: &mut P,
    root: &RootMetadata,
    context: RoleVerificationContext<'_, P::Output>,
) -> Result<StreamedRoleInfo, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    let policy = role(root, expected_role)?;
    let RoleVerificationContext {
        output,
        role_verifier,
        input: RoleVerificationInput { chunk, progress },
    } = context;
    let captured = capture_role_into(storage, document, expected_role, chunk, parser, output)
        .map_err(map_role_error)?;
    verify_captured_role(
        storage,
        document,
        expected_role,
        RoleVerificationPolicy {
            role: policy,
            keys: &root.keys[..usize::from(root.key_count)],
        },
        captured,
        role_verifier,
        RoleVerificationInput { chunk, progress },
    )
    .map_err(map_role_error)
}
