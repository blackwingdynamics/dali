use dali_metadata::BundleManifestSummary;
use super::RepositoryGenerationAdmission;

#[inline(never)]
/// Internal helper for `admit_bundle_manifest`.
fn admit_bundle_manifest<S>(
    storage: &mut S,
    root: &RootMetadata,
    request: RepositoryLoadRequest,
    chunk: &mut [u8],
    metadata: &mut RepositoryMetadataScratch,
    verifier: &mut RepositoryScratch,
    progress: fn() -> bool,
) -> Result<BundleManifestSummary, BinaryRepositoryError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    let parser = reset_parser(
        // SAFETY: the bundle parser variant is active for this sequential phase.
        unsafe { metadata.bundle_parser() },
        BinaryBundleBodyStreamParser::new(),
    );
    let mut output = MaybeUninit::uninit();
    verify_role_from_root(
        storage,
        crate::storage::repository::RepositoryDocument::Bundle,
        MetadataRole::Bundle,
        parser,
        root,
        RoleVerificationContext {
            output: &mut output,
            role_verifier: unsafe { verifier.target_verifier() },
            input: RoleVerificationInput { chunk, progress },
        },
    )
    .map_err(map_bundle_error)?;
    // SAFETY: verify_role_from_root initializes the summary before returning.
    let summary = unsafe { output.assume_init() };
    validate_bundle_admission(summary, request)?;
    Ok(summary)
}

/// Internal helper for `map_bundle_error`.
fn map_bundle_error<E>(error: BinaryRepositoryError<E>) -> BinaryRepositoryError<E> {
    match error {
        BinaryRepositoryError::RoleStorage(error) => BinaryRepositoryError::BundleRoleStorage(error),
        BinaryRepositoryError::RoleDecode => BinaryRepositoryError::BundleRoleDecode,
        BinaryRepositoryError::RoleSignature => BinaryRepositoryError::BundleRoleSignature,
        other => other,
    }
}

/// Internal helper for `validate_bundle_admission`.
fn validate_bundle_admission<E>(
    summary: BundleManifestSummary,
    request: RepositoryLoadRequest,
) -> Result<(), BinaryRepositoryError<E>> {
    if summary.target_profile != request.target_profile {
        return Err(BinaryRepositoryError::BundleTargetMismatch);
    }
    let Some(committed) = request.committed_generation else {
        return Ok(());
    };
    match request.generation_admission {
        RepositoryGenerationAdmission::ActiveBoot if summary.header.version < committed.version => {
            Err(BinaryRepositoryError::BundleGenerationRollback)
        }
        RepositoryGenerationAdmission::ActiveBoot if summary.header.version > committed.version => {
            Err(BinaryRepositoryError::BundleGenerationAhead)
        }
        RepositoryGenerationAdmission::ActiveBoot => Ok(()),
        RepositoryGenerationAdmission::AuthorizedCandidate
            if summary.header.version <= committed.version =>
        {
            Err(BinaryRepositoryError::BundleGenerationRollback)
        }
        RepositoryGenerationAdmission::AuthorizedCandidate => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(admission: RepositoryGenerationAdmission) -> RepositoryLoadRequest {
        RepositoryLoadRequest {
            package_id: None,
            target_profile: dali_metadata::BoundedText::new("f405").expect("profile fits"),
            contract: None,
            now: None,
            committed_generation: dali_metadata::TrustStoreRecord::new(
                2,
                dali_metadata::Sha256Digest([1; dali_metadata::SHA256_LENGTH]),
            ),
            generation_admission: admission,
        }
    }

    fn summary(version: u64) -> BundleManifestSummary {
        BundleManifestSummary {
            header: dali_metadata::MetadataHeader {
                role: dali_metadata::MetadataRole::Bundle,
                version,
                expires: 0,
            },
            target_profile: dali_metadata::BoundedText::new("f405").expect("profile fits"),
            file_count: 7,
        }
    }

    #[test]
    fn active_boot_accepts_the_committed_generation() {
        assert!(validate_bundle_admission::<()>(
            summary(2),
            request(RepositoryGenerationAdmission::ActiveBoot)
        )
        .is_ok());
    }

    #[test]
    fn active_boot_rejects_an_older_generation() {
        assert!(matches!(
            validate_bundle_admission::<()>(summary(1), request(RepositoryGenerationAdmission::ActiveBoot)),
            Err(BinaryRepositoryError::BundleGenerationRollback)
        ));
    }

    #[test]
    fn candidate_installation_requires_strictly_newer_generation() {
        assert!(matches!(
            validate_bundle_admission::<()>(summary(2), request(RepositoryGenerationAdmission::AuthorizedCandidate)),
            Err(BinaryRepositoryError::BundleGenerationRollback)
        ));
    }
}
