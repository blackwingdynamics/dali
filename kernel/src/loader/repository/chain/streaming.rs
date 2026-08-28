/// Internal helper for `is_revoked`.
fn is_revoked(
    revocations: &dali_metadata::RevocationMetadata,
    delegation: &dali_metadata::DelegationMetadata,
    current_version: u64,
) -> bool {
    revocations
        .records
        .iter()
        .take(usize::from(revocations.record_count))
        .any(|record| {
            record.developer_id == delegation.developer_id
                && record.key_id == delegation.key_id
                && record.effective_version <= current_version
        })
}

/// Parses and verifies Root metadata against a target-provisioned anchor.
#[inline(never)]
pub(crate) fn stream_verified_root<S>(
    storage: &mut S,
    anchors: &[dali_targets::TrustAnchorProfile],
    chunk: &mut [u8],
    output: &mut MaybeUninit<RootMetadata>,
    scratch: &mut RepositoryScratch,
    progress: fn() -> bool,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let parser = reset_parser(scratch.root_parser(), BinaryRootBodyStreamParser::new());
    let captured = capture_role_into(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        chunk,
        parser,
        output,
    )?;
    // SAFETY: capture_role_into writes the parser output before returning.
    let root = unsafe { output.assume_init_ref() };
    if !anchors
        .iter()
        .copied()
        .any(|anchor| super::trust::contains_root_anchor(root, anchor))
    {
        return Err(StreamedRoleError::UnknownTrustAnchor);
    }
    let role = root
        .roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|definition| definition.role == MetadataRole::Root)
        .copied()
        .ok_or(StreamedRoleError::Decode)?;
    verify_captured_role(
        storage,
        RepositoryDocument::Root,
        MetadataRole::Root,
        RoleVerificationPolicy {
            role,
            keys: &root.keys[..usize::from(root.key_count)],
        },
        captured,
        unsafe { scratch.target_verifier() },
        RoleVerificationInput { chunk, progress },
    )
}

/// Captures typed metadata and its complete serialized digest without verifying it.
#[inline(never)]
pub(crate) fn capture_role_into<S, P>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    chunk: &mut [u8],
    parser: &mut P,
    output: &mut MaybeUninit<P::Output>,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
    P: BinaryRoleBodyParser,
{
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Reading repository role: {:?}", document),
    );
    let mut envelope = BinaryEnvelopeStreamParser::new(expected_role);
    let mut digest = dali_crypto::Sha256Accumulator::new();
    let mut decode_error = false;
    let length = storage
        .stream_metadata(document, chunk, |bytes| {
            digest.update(bytes);
            if decode_error {
                return Ok(());
            }
            if envelope
                .feed(bytes, |body| {
                    parser.feed(body).map_err(|_| DecodeError::InvalidValue)
                })
                .is_err()
            {
                decode_error = true;
            }
            Ok(())
        })
        .map_err(StreamedRoleError::Storage)?;
    crate::logging::info(
        crate::logging::BOOT_SUBSYSTEM,
        format_args!("[LOADER] Repository role read complete: {:?}, {} bytes", document, length),
    );
    if decode_error {
        return Err(StreamedRoleError::Decode);
    }
    let envelope = envelope.finish().map_err(|_| StreamedRoleError::Decode)?;
    parser
        .finish_into(output)
        .map_err(|_| StreamedRoleError::Decode)?;
    let delivered = envelope_total_length(envelope);
    if delivered != u64::from(length) {
        return Err(StreamedRoleError::LengthMismatch);
    }
    Ok(StreamedRoleInfo {
        envelope,
        length,
        digest: Sha256Digest(digest.finalize()),
    })
}

#[inline(never)]
/// Internal helper for `verify_captured_role`.
fn verify_captured_role<S>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    expected_role: MetadataRole,
    policy: RoleVerificationPolicy<'_>,
    captured: StreamedRoleInfo,
    verifier_workspace: &mut MaybeUninit<StreamingRoleVerifier>,
    input: RoleVerificationInput<'_>,
) -> Result<StreamedRoleInfo, StreamedRoleError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let RoleVerificationInput { chunk, progress } = input;
    StreamingRoleVerifier::initialize(
        verifier_workspace,
        policy.role,
        policy.keys,
        captured.envelope.signatures,
    )
    .map_err(|_| StreamedRoleError::Signature)?;
    let mut replay = BinaryEnvelopeStreamParser::new(expected_role);
    let mut digest = dali_crypto::Sha256Accumulator::new();
    let mut decode_error = false;
    // SAFETY: initialize() completed successfully and this workspace is
    // exclusively borrowed for the replay pass.
    let verifier = unsafe { verifier_workspace.assume_init_mut() };
    storage
        .stream_metadata(document, chunk, |bytes| {
            if decode_error {
                return Ok(());
            }
            digest.update(bytes);
            if replay
                .feed(bytes, |body| {
                    verifier
                        .update_with_progress(body, progress)
                        .map_err(|_| DecodeError::InvalidValue)
                })
                .is_err()
            {
                decode_error = true;
            }
            Ok(())
        })
        .map_err(StreamedRoleError::Storage)?;
    let replayed = replay.finish().map_err(|_| StreamedRoleError::Decode)?;
    if decode_error
        || replayed.body_sha256 != captured.envelope.body_sha256
        || Sha256Digest(digest.finalize()) != captured.digest
    {
        return Err(StreamedRoleError::Decode);
    }
    verifier
        .finish_in_place()
        .map_err(|_| StreamedRoleError::Signature)?;
    Ok(captured)
}

/// Internal helper for `envelope_total_length`.
fn envelope_total_length(envelope: StreamedEnvelope) -> u64 {
    u64::from(envelope.body_length)
        .saturating_add(dali_metadata::BINARY_ENVELOPE_HEADER_BYTES as u64)
        .saturating_add(
            u64::from(envelope.signatures.count)
                .saturating_mul(dali_metadata::BINARY_SIGNATURE_RECORD_BYTES as u64),
        )
}
