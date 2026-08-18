//! Real filesystem discovery for AMRN format 4 packages.

use dali_amrn::v4;

use crate::{
    drivers::{BlockDeviceRef, StorageError},
    loader_contract::PackageCatalog,
    storage::filesystem,
};

use super::execution::LoadedApplication;

pub(crate) fn load_files<D>(
    device: &D,
    slot_manager: &mut crate::runtime::slots::SlotManager,
) -> Result<LoadedApplication, super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut catalog = PackageCatalog::<{ filesystem::MAX_ROOT_AMRN_FILES }>::new();
    let target = crate::platform::TARGET_PROFILE;
    let isolation = target
        .memory
        .isolation
        .ok_or(super::LoaderError::UnsupportedCurrentAbiTarget)?;
    let device = BlockDeviceRef::new(device);
    filesystem::with_amrn_files(device, |file| {
        let header_bytes = super::identity::read_header(&file)?;
        let (header, _contract, slot) = crate::loader_contract::select_slot(
            &header_bytes,
            target.amrn_target_id,
            isolation.slots,
        )
        .map_err(|_| super::identity::v4_error(v4::Error::InvalidHeader))?;
        catalog
            .register(header, slot, slot_manager.is_reserved(slot))
            .map_err(super::LoaderError::PackageCatalog)
    })
    .map_err(super::LoaderError::Filesystem)?
    .map_err(|error| error)?;

    let selected = catalog
        .select()
        .ok_or(super::identity::v4_error(v4::Error::InvalidHeader))?;
    let selected_identity = selected.header.metadata.package_id;
    let mut loaded = None;
    filesystem::with_amrn_files(device, |file| {
        let header = super::identity::read_header(&file)?;
        let identity_end = v4::PACKAGE_ID_OFFSET + selected_identity.len();
        if header[v4::PACKAGE_ID_OFFSET..identity_end] != selected_identity {
            return Ok(());
        }
        file.rewind().map_err(super::LoaderError::Filesystem)?;
        loaded = Some(super::identity::load_file(file, slot_manager)?);
        Ok(())
    })
    .map_err(super::LoaderError::Filesystem)?
    .map_err(|error| error)?;
    loaded.ok_or(super::identity::v4_error(v4::Error::InvalidHeader))
}
