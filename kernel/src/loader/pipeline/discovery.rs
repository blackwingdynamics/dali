//! Real filesystem discovery for AMRN format 4 packages.

use dali_amrn::v4;

use crate::{
    drivers::{BlockDeviceRef, StorageError},
    loader_contract::PackageCatalog,
    storage::filesystem,
};

use super::execution::LoadedApplications;

pub(crate) fn load_files<D>(
    device: &D,
    slot_manager: &mut crate::runtime::slots::SlotManager,
) -> Result<LoadedApplications<{ filesystem::MAX_ROOT_AMRN_FILES }>, super::LoaderError>
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

    let mut loaded = LoadedApplications::new();
    let mut previous_slot = None;
    for _ in 0..catalog.len() {
        let selected = catalog
            .select_after(previous_slot)
            .ok_or(super::identity::v4_error(v4::Error::InvalidHeader))?;
        let selected_identity = selected.header.metadata.package_id;
        load_selected_file(device, selected_identity, slot_manager, &mut loaded)?;
        previous_slot = Some(selected.slot.id);
    }
    Ok(loaded)
}

fn load_selected_file<D>(
    device: D,
    selected_identity: [u8; 16],
    slot_manager: &mut crate::runtime::slots::SlotManager,
    loaded: &mut LoadedApplications<{ filesystem::MAX_ROOT_AMRN_FILES }>,
) -> Result<(), super::LoaderError>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut found = false;
    filesystem::with_amrn_files(device, |file| {
        let header = super::identity::read_header(&file)?;
        let identity_end = v4::PACKAGE_ID_OFFSET + selected_identity.len();
        if header[v4::PACKAGE_ID_OFFSET..identity_end] != selected_identity {
            return Ok(());
        }
        file.rewind().map_err(super::LoaderError::Filesystem)?;
        let application = super::identity::load_file(file, slot_manager)?;
        found = true;
        if !loaded.push(application) {
            return Err(super::LoaderError::PackageCatalog(
                crate::loader_contract::CatalogError::CapacityExceeded,
            ));
        }
        Ok(())
    })
    .map_err(super::LoaderError::Filesystem)?
    .map_err(|error| error)?;
    if found {
        Ok(())
    } else {
        Err(super::identity::v4_error(v4::Error::InvalidHeader))
    }
}
