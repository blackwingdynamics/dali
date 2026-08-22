use super::*;

struct EmptyRepository;

impl RepositoryStreamStorage for EmptyRepository {
    type Error = ();

    fn stream_metadata<F>(
        &mut self,
        _: RepositoryDocument<'_>,
        output: &mut [u8],
        _: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>,
    {
        let _ = output;
        Ok(u32::MAX)
    }

    fn stream_package<F>(
        &mut self,
        _: RepositoryPackageDigest,
        output: &mut [u8],
        _: F,
    ) -> Result<u32, Self::Error>
    where
        F: FnMut(&[u8]) -> Result<(), Self::Error>,
    {
        let _ = output;
        Ok(u32::MAX)
    }
}

#[test]
fn rejects_storage_lengths_larger_than_caller_buffers() {
    let mut storage = EmptyRepository;
    let mut buffers = RepositoryBuffers::default();
    let result = load_repository(
        &mut storage,
        RepositoryLoadRequest {
            package_id: Some(PackageId([1; dali_metadata::KEY_ID_LENGTH])),
            target_profile: dali_metadata::BoundedText::new("test").expect("test profile fits"),
            contract: Some(Contract {
                target_id: 0,
                code_load_address: 0,
                code_capacity: 0,
                data_load_address: 0,
                data_capacity: 0,
            }),
            now: None,
            committed_generation: None,
            generation_admission: RepositoryGenerationAdmission::ActiveBoot,
        },
        &mut buffers,
    );
    assert_eq!(result, Err(RepositoryLoaderError::ArtifactTooLarge));
}

#[test]
fn rejects_missing_target_records_before_delegation_read() {
    let targets = dali_metadata::TargetsMetadata {
        header: dali_metadata::MetadataHeader {
            role: dali_metadata::MetadataRole::Targets,
            version: 1,
            expires: 0,
        },
        delegations: [dali_metadata::BoundedText::default(); dali_metadata::MAX_DELEGATION_SCOPES],
        delegation_count: 0,
        packages: [TargetPackage::default(); dali_metadata::MAX_TARGET_RECORDS],
        package_count: 0,
    };
    assert_eq!(
        find_target::<()>(&targets, PackageId([2; dali_metadata::KEY_ID_LENGTH])),
        Err(RepositoryLoaderError::MissingRecord)
    );
}
