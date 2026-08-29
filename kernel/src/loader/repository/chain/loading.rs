/// Revocation -> AMRN for all bounded matching cartridges.
/// Verifies Root -> Timestamp -> Snapshot -> Targets -> Delegation ->
/// Revocation -> AMRN for all bounded matching cartridges.
pub fn load_binary_repository<S>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
) -> Result<BinaryRepositoryAuthorizations, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let Some(contract) = request.contract else {
        return Err(BinaryRepositoryError::CartridgeInvalidHeader(
            dali_amrn::v5::Error::InvalidHeader,
        ));
    };
    load_binary_repository_with_contract(
        storage,
        request,
        anchors,
        buffers,
        |_| Some(contract),
        no_repository_progress,
    )
}

/// Internal helper for `no_repository_progress`.
fn no_repository_progress() -> bool {
    true
}

/// Verifies a repository and resolves the AMRN memory contract after target selection.
pub fn load_binary_repository_with_contract<S, F>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    anchors: &[dali_targets::TrustAnchorProfile],
    buffers: &mut BinaryRepositoryBuffers,
    mut contract_for: F,
    progress: fn() -> bool,
) -> Result<BinaryRepositoryAuthorizations, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    F: FnMut(dali_metadata::TargetCartridge) -> Option<dali_amrn::v3::Contract>,
{
    stream_verified_root(
        storage,
        anchors,
        &mut buffers.chunk,
        &mut buffers.root,
        &mut buffers.scratch,
        progress,
    )
    .map_err(map_root_error)?;
    // SAFETY: stream_verified_root writes the root before returning and this
    // workspace is exclusively borrowed for the duration of this load.
    let root = unsafe { buffers.root.assume_init_ref() };
    let bundle = admit_bundle_manifest(
        storage,
        root,
        request,
        &mut buffers.chunk,
        &mut buffers.metadata,
        &mut buffers.scratch,
        progress,
    )?;
    verify_timestamp_and_snapshot(
        storage,
        root,
        &mut buffers.chunk,
        unsafe { buffers.metadata.snapshot() },
        unsafe { buffers.metadata_parser.snapshot() },
        unsafe { buffers.scratch.target_verifier() },
        progress,
    )?;
    // SAFETY: the preceding helper writes the snapshot before this reference
    // is used, and the workspace remains exclusively borrowed by this load.
    let snapshot = unsafe { (*buffers.metadata.snapshot).assume_init_ref() };
    let targets_role = role(root, MetadataRole::Targets)?;
    let targets = select_targets_into(
        storage,
        request,
        root,
        targets_role,
        &mut buffers.chunk,
        // SAFETY: the target verifier is exclusively used by this phase.
        unsafe { buffers.scratch.target_verifier() },
        progress,
    )?;
    let first_target = targets
        .iter()
        .next()
        .ok_or(BinaryRepositoryError::MissingRecord)?;
    if !same_reference(
        snapshot.targets.version,
        snapshot.targets.length,
        snapshot.targets.sha256,
        first_target.version,
        first_target.length,
        first_target.digest,
    ) {
        return Err(BinaryRepositoryError::TargetsReferenceMismatch);
    }
    verify_revocations(
        storage,
        root,
        snapshot,
        RevocationVerificationContext {
            chunk: &mut buffers.chunk,
            output: &mut buffers.revocations,
            parser: unsafe { buffers.metadata_parser.revocations() },
            role_verifier: unsafe { buffers.scratch.target_verifier() },
            progress,
        },
    )?;
    // SAFETY: the preceding helper writes the revocation metadata before this
    // reference is used, and the workspace remains exclusively borrowed here.
    let revocations = unsafe { buffers.revocations.assume_init_ref() };
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Revocations accepted; reconstructing trust state"),
    );
    let security_state =
        dali_metadata::TrustStoreSecurityState::from_verified_metadata(root, revocations)
            .map_err(|_| BinaryRepositoryError::SecurityState)?;
    crate::logging::info(
        crate::logging::SECURITY_SUBSYSTEM,
        format_args!("[SECURITY] Trust state reconstructed"),
    );
    collect_delegation_references(snapshot, &targets, &mut buffers.delegation_references)?;
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Delegation references collected"),
    );
    let mut authorizations = verify_cartridges_into(
        storage,
        root,
        revocations,
        &targets,
        buffers.delegation_references,
        &mut CartridgeVerificationPass {
            chunk: &mut buffers.chunk,
            amrn_buffers: &mut buffers.amrn,
            // SAFETY: the delegation output is exclusively used by one
            // cartridge verification at a time.
            delegation_output: unsafe { buffers.metadata.delegation() },
            // SAFETY: the verifier workspace is exclusively used by this
            // phase.
            role_verifier: unsafe { buffers.scratch.target_verifier() },
            contract_for: &mut contract_for,
            progress,
        },
    )?;
    authorizations.security_state = Some(security_state);
    authorizations.committed_generation = request.committed_generation;
    authorizations.bundle_generation = Some(bundle.header.version);
    Ok(authorizations)
}

/// Internal helper for `select_targets_into`.
fn select_targets_into<S>(
    storage: &mut S,
    request: RepositoryLoadRequest,
    root: &RootMetadata,
    targets_role: RoleDefinition,
    chunk: &mut [u8],
    verifier_workspace: &mut MaybeUninit<StreamingRoleVerifier>,
    progress: fn() -> bool,
) -> Result<
    streaming::VerifiedBinaryTargets<MAX_BINARY_REPOSITORY_CARTRIDGES>,
    BinaryRepositoryError<S::Error>,
>
where
    S: RepositoryStreamStorage,
{
    let targets = if let Some(cartridge_id) = request.cartridge_id {
        streaming::select_verified_binary_targets_for_cartridge::<S, MAX_BINARY_REPOSITORY_CARTRIDGES>(
            storage,
            cartridge_id,
            targets_role,
            &root.keys[..usize::from(root.key_count)],
            chunk,
            progress,
            verifier_workspace,
        )
    } else {
        streaming::select_verified_binary_targets::<S, MAX_BINARY_REPOSITORY_CARTRIDGES>(
            storage,
            request.target_profile,
            targets_role,
            &root.keys[..usize::from(root.key_count)],
            chunk,
            progress,
            verifier_workspace,
        )
    }
    .map_err(map_targets_error)?;
    Ok(targets)
}

/// Internal helper for `collect_delegation_references`.
fn collect_delegation_references<E>(
    snapshot: &dali_metadata::SnapshotMetadata,
    targets: &streaming::VerifiedBinaryTargets<MAX_BINARY_REPOSITORY_CARTRIDGES>,
    output: &mut [Option<dali_metadata::DelegationReference>; MAX_BINARY_REPOSITORY_CARTRIDGES],
) -> Result<(), BinaryRepositoryError<E>> {
    *output = [None; MAX_BINARY_REPOSITORY_CARTRIDGES];
    for (index, selected) in targets.iter().enumerate() {
        let delegation_id = selected
            .target
            .delegation_id
            .as_str()
            .ok_or(BinaryRepositoryError::MissingRecord)?;
        output[index] = Some(
            snapshot
                .delegations
                .iter()
                .take(usize::from(snapshot.delegation_count))
                .find(|reference| reference.id.as_str() == Some(delegation_id))
                .copied()
                .ok_or(BinaryRepositoryError::MissingRecord)?,
        );
    }
    Ok(())
}

/// Internal helper for `verify_cartridges_into`.
fn verify_cartridges_into<S, F>(
    storage: &mut S,
    root: &RootMetadata,
    revocations: &dali_metadata::RevocationMetadata,
    targets: &streaming::VerifiedBinaryTargets<MAX_BINARY_REPOSITORY_CARTRIDGES>,
    delegation_references: [Option<dali_metadata::DelegationReference>;
        MAX_BINARY_REPOSITORY_CARTRIDGES],
    pass: &mut CartridgeVerificationPass<'_, F>,
) -> Result<BinaryRepositoryAuthorizations, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
    F: FnMut(dali_metadata::TargetCartridge) -> Option<dali_amrn::v3::Contract>,
{
    let mut authorizations = BinaryRepositoryAuthorizations::new();
    for (index, selected) in targets.iter().enumerate() {
        let contract =
            (pass.contract_for)(selected.target).ok_or(BinaryRepositoryError::CartridgeContract)?;
        let developer_public_key = verify_delegation_and_cartridge(
            &mut CartridgeVerificationContext {
                storage,
                root,
                revocations,
                chunk: pass.chunk,
                amrn_buffers: pass.amrn_buffers,
                delegation_reference: delegation_references[index]
                    .ok_or(BinaryRepositoryError::MissingRecord)?,
                delegation_output: pass.delegation_output,
                role_verifier: pass.role_verifier,
                progress: pass.progress,
            },
            selected.target,
            selected.version,
            contract,
        )?;
        authorizations
            .push(BinaryRepositoryAuthorization {
                target: selected.target,
                developer_public_key,
            })
            .map_err(|_| {
                BinaryRepositoryError::CartridgeInvalidHeader(dali_amrn::v5::Error::InvalidHeader)
            })?;
    }
    Ok(authorizations)
}

/// Internal implementation state for `CartridgeVerificationPass`.
struct CartridgeVerificationPass<'a, F> {
/// Internal field `chunk`.
    chunk: &'a mut [u8],
/// Internal field `amrn_buffers`.
    amrn_buffers: &'a mut amrn::AmrnStreamBuffers,
/// Internal field `delegation_output`.
    delegation_output: &'a mut MaybeUninit<dali_metadata::DelegationMetadata>,
/// Internal field `role_verifier`.
    role_verifier: &'a mut MaybeUninit<StreamingRoleVerifier>,
/// Internal field `contract_for`.
    contract_for: &'a mut F,
/// Internal field `progress`.
    progress: fn() -> bool,
}
