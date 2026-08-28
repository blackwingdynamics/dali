//! Bounded identity and slot selection for validated AMRN v4 metadata.

use dali_amrn::v4;
use dali_targets::IsolationSlot;

/// A validated package header paired with its manifest-owned slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscoveredPackage {
    /// The package header obtained from the real package source.
    pub header: v4::Header,
    /// The manifest slot accepted by the header.
    pub slot: IsolationSlot,
}

/// Errors returned while building a bounded package catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogError {
    /// The package identity contains no non-zero byte.
    InvalidIdentity,
    /// The header names a slot different from the selected manifest slot.
    SlotMismatch,
    /// Another package already owns the same identity.
    DuplicateIdentity,
    /// The selected slot is already occupied or catalogued.
    SlotOccupied,
    /// The fixed catalog capacity has been reached.
    CapacityExceeded,
}

/// Fixed-capacity catalog used before package loading begins.
pub struct PackageCatalog<const CAPACITY: usize> {
    /// Stores the `entries` value for this bounded state.
    entries: [Option<DiscoveredPackage>; CAPACITY],
    /// Stores the `length` value for this bounded state.
    length: usize,
}

impl<const CAPACITY: usize> PackageCatalog<CAPACITY> {
    /// Creates an empty catalog without allocating.
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            length: 0,
        }
    }

    /// Returns whether no package candidate has been accepted.
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Returns the number of accepted package candidates.
    #[cfg(not(feature = "repository-loader"))]
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Registers one already-validated package header.
    pub fn register(
        &mut self,
        header: v4::Header,
        slot: IsolationSlot,
        slot_occupied: bool,
    ) -> Result<(), CatalogError> {
        if header.metadata.package_id.iter().all(|byte| *byte == 0) {
            return Err(CatalogError::InvalidIdentity);
        }
        if header.metadata.slot_id != slot.id {
            return Err(CatalogError::SlotMismatch);
        }
        if self.has_identity(header.metadata.package_id) {
            return Err(CatalogError::DuplicateIdentity);
        }
        if slot_occupied || self.has_slot(slot.id) {
            return Err(CatalogError::SlotOccupied);
        }
        let Some(entry) = self.entries.get_mut(self.length) else {
            return Err(CatalogError::CapacityExceeded);
        };
        *entry = Some(DiscoveredPackage { header, slot });
        self.length += 1;
        Ok(())
    }

    /// Selects the lowest manifest slot independently of discovery order.
    pub fn select(&self) -> Option<DiscoveredPackage> {
        self.select_after(None)
    }

    /// Selects the lowest manifest slot after an optional previously selected slot.
    pub fn select_after(&self, previous_slot: Option<u8>) -> Option<DiscoveredPackage> {
        if self.is_empty() {
            return None;
        }
        self.entries[..self.length]
            .iter()
            .flatten()
            .copied()
            .filter(|candidate| previous_slot.is_none_or(|slot_id| candidate.slot.id > slot_id))
            .min_by_key(|candidate| candidate.slot.id)
    }

    /// Finds a package by its stable identity.
    pub fn find_by_identity(&self, package_id: [u8; 16]) -> Option<DiscoveredPackage> {
        self.entries[..self.length]
            .iter()
            .flatten()
            .find(|candidate| candidate.header.metadata.package_id == package_id)
            .copied()
    }

    /// Performs the `has_identity` operation for this subsystem.
    fn has_identity(&self, package_id: [u8; 16]) -> bool {
        self.find_by_identity(package_id).is_some()
    }

    /// Performs the `has_slot` operation for this subsystem.
    fn has_slot(&self, slot_id: u8) -> bool {
        self.entries[..self.length]
            .iter()
            .flatten()
            .any(|candidate| candidate.slot.id == slot_id)
    }
}

impl<const CAPACITY: usize> Default for PackageCatalog<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dali_amrn::v3;

    const CATALOG_CAPACITY: usize = 2;
    const TARGET_ID: u8 = 2;
    const SLOT0_ID: u8 = 0;
    const SLOT1_ID: u8 = 1;
    const CODE_ORIGIN: u32 = 0x2000_8000;
    const DATA_ORIGIN: u32 = 0x2000_c000;
    const SLOT_LENGTH: u32 = 0x4000;
    const STACK_LENGTH: u32 = 0x1000;

    const SLOT0: IsolationSlot = IsolationSlot {
        id: SLOT0_ID,
        name: "slot0",
        code_origin: CODE_ORIGIN,
        code_length: SLOT_LENGTH,
        data_origin: DATA_ORIGIN,
        data_length: SLOT_LENGTH,
        stack_length: STACK_LENGTH,
    };

    const SLOT1: IsolationSlot = IsolationSlot {
        id: SLOT1_ID,
        name: "slot1",
        code_origin: CODE_ORIGIN + SLOT_LENGTH,
        code_length: SLOT_LENGTH,
        data_origin: DATA_ORIGIN + SLOT_LENGTH,
        data_length: SLOT_LENGTH,
        stack_length: STACK_LENGTH,
    };

    const SLOT2: IsolationSlot = IsolationSlot {
        id: 2,
        name: "slot2",
        ..SLOT1
    };

    fn header(identity_byte: u8, slot_id: u8) -> v4::Header {
        v4::Header {
            image: v3::Header {
                target_id: TARGET_ID,
                code_size: 4,
                data_init_size: 0,
                data_zero_size: 0,
                stack_size: STACK_LENGTH,
                linked_code_base: CODE_ORIGIN,
                linked_data_base: DATA_ORIGIN,
                code_load_address: CODE_ORIGIN,
                data_load_address: DATA_ORIGIN,
                execution_offset: 0,
                relocation_offset: v4::HEADER_SIZE as u32 + 4,
                relocation_count: 0,
                crc32: 0,
            },
            metadata: v4::Metadata {
                package_id: [identity_byte; 16],
                package_version: v4::Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                minimum_kernel_version: v4::Version {
                    major: 0,
                    minor: 1,
                    patch: 0,
                },
                required_services: 0,
                slot_id,
            },
            package_crc32: 0,
        }
    }

    #[test]
    fn selection_is_independent_of_discovery_order() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();
        catalog
            .register(header(2, SLOT1_ID), SLOT1, false)
            .expect("slot 1 registers");
        catalog
            .register(header(1, SLOT0_ID), SLOT0, false)
            .expect("slot 0 registers");

        let selected = catalog.select().expect("a package is selected");
        assert_eq!(selected.slot.id, SLOT0_ID);
        assert_eq!(selected.header.metadata.package_id, [1; 16]);
    }

    #[test]
    fn selects_each_declared_slot_in_manifest_order() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();
        catalog
            .register(header(2, SLOT1_ID), SLOT1, false)
            .expect("slot 1 registers");
        catalog
            .register(header(1, SLOT0_ID), SLOT0, false)
            .expect("slot 0 registers");

        let first = catalog
            .select_after(None)
            .expect("first package is selected");
        let second = catalog
            .select_after(Some(first.slot.id))
            .expect("second package is selected");

        assert_eq!(first.slot.id, SLOT0_ID);
        assert_eq!(second.slot.id, SLOT1_ID);
        assert!(catalog.select_after(Some(second.slot.id)).is_none());
    }

    #[test]
    fn rejects_duplicate_identity() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();
        catalog
            .register(header(1, SLOT0_ID), SLOT0, false)
            .expect("first package registers");

        assert_eq!(
            catalog.register(header(1, SLOT1_ID), SLOT1, false),
            Err(CatalogError::DuplicateIdentity)
        );
    }

    #[test]
    fn rejects_duplicate_and_external_slot_occupancy() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();
        catalog
            .register(header(1, SLOT0_ID), SLOT0, false)
            .expect("first package registers");

        assert_eq!(
            catalog.register(header(2, SLOT0_ID), SLOT0, false),
            Err(CatalogError::SlotOccupied)
        );
        assert_eq!(
            catalog.register(header(3, SLOT1_ID), SLOT1, true),
            Err(CatalogError::SlotOccupied)
        );
    }

    #[test]
    fn rejects_a_header_for_the_wrong_manifest_slot() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();

        assert_eq!(
            catalog.register(header(1, SLOT1_ID), SLOT0, false),
            Err(CatalogError::SlotMismatch)
        );
    }

    #[test]
    fn rejects_capacity_overflow() {
        let mut catalog = PackageCatalog::<CATALOG_CAPACITY>::new();
        catalog
            .register(header(1, SLOT0_ID), SLOT0, false)
            .expect("slot 0 registers");
        catalog
            .register(header(2, SLOT1_ID), SLOT1, false)
            .expect("slot 1 registers");

        assert_eq!(
            catalog.register(header(3, 2), SLOT2, false),
            Err(CatalogError::CapacityExceeded)
        );
    }
}
