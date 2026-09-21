//! Manifest-driven application slot allocation.

use dali_targets::IsolationSlot;

/// Defines the SLOT MASK BITS used by this module.
const SLOT_MASK_BITS: usize = u32::BITS as usize;

/// Addressable memory segment owned by an application slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotRegion {
    /// Executable code and read-only application data.
    Code,
    /// Writable initialized, zero-initialized, and PSP data.
    Data,
}

/// A selected application slot and its stable manifest index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotAllocation {
    /// Stores the index associated with this bounded state.
    index: usize,
    /// Stores the slot associated with this bounded state.
    slot: IsolationSlot,
}

impl SlotAllocation {
    /// Returns the manifest index of the selected slot.
    pub const fn index(self) -> usize {
        self.index
    }

    /// Returns the manifest-owned boundaries of the selected slot.
    pub const fn slot(self) -> IsolationSlot {
        self.slot
    }

    /// Returns whether a complete range belongs to this allocation's segment.
    pub fn contains(self, region: SlotRegion, start: u32, length: u32) -> bool {
        let (origin, capacity) = match region {
            SlotRegion::Code => (self.slot.code_origin, self.slot.code_length),
            SlotRegion::Data => (self.slot.data_origin, self.slot.data_length),
        };
        let end = match start.checked_add(length) {
            Some(end) => end,
            None => return false,
        };
        start >= origin && end <= origin.saturating_add(capacity)
    }
}

/// Errors returned while constructing or releasing a slot manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotManagerError {
    /// The caller-provided fixed capacity cannot hold the manifest slots.
    CapacityExceeded,
    /// The target does not declare the requested slot.
    UndeclaredSlot,
    /// The requested slot is already reserved by another application.
    SlotOccupied,
    /// The allocation was not active or did not match the manifest entry.
    InvalidAllocation,
}

/// Fixed-capacity allocator for manifest-declared application slots.
///
/// The manager owns only allocation state. Slot boundaries remain immutable
/// values supplied by the target manifest; no address arithmetic or filename
/// convention can create a slot.
pub struct SlotManager {
    /// Stores the slots associated with this bounded state.
    slots: &'static [IsolationSlot],
    /// Stores the occupied associated with this bounded state.
    occupied: u32,
}

impl SlotManager {
    /// Creates a manager for a manifest-owned slot table.
    pub const fn new(slots: &'static [IsolationSlot]) -> Result<Self, SlotManagerError> {
        if slots.len() > SLOT_MASK_BITS {
            return Err(SlotManagerError::CapacityExceeded);
        }
        Ok(Self { slots, occupied: 0 })
    }

    /// Returns the number of manifest slots managed by this instance.
    pub const fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns whether the manifest declares no slots.
    pub const fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Reserves the first free manifest slot.
    pub fn allocate(&mut self) -> Option<SlotAllocation> {
        for (index, slot) in self.slots.iter().copied().enumerate() {
            let mask = 1_u32 << index;
            if self.occupied & mask == 0 {
                self.occupied |= mask;
                return Some(SlotAllocation { index, slot });
            }
        }
        None
    }

    /// Reserves the exact manifest slot selected by a validated cartridge.
    pub fn reserve(&mut self, slot: IsolationSlot) -> Result<SlotAllocation, SlotManagerError> {
        let Some((index, _)) = self
            .slots
            .iter()
            .copied()
            .enumerate()
            .find(|(_, declared)| *declared == slot)
        else {
            return Err(SlotManagerError::UndeclaredSlot);
        };
        let mask = 1_u32 << index;
        if self.occupied & mask != 0 {
            return Err(SlotManagerError::SlotOccupied);
        }
        self.occupied |= mask;
        Ok(SlotAllocation { index, slot })
    }

    /// Returns whether a manifest slot is currently reserved.
    pub fn is_reserved(&self, slot: IsolationSlot) -> bool {
        self.slots
            .iter()
            .copied()
            .enumerate()
            .find(|(_, declared)| *declared == slot)
            .map(|(index, _)| self.occupied & (1_u32 << index) != 0)
            .unwrap_or(false)
    }

    /// Releases an allocation after its application has stopped.
    pub fn release(&mut self, allocation: SlotAllocation) -> Result<(), SlotManagerError> {
        let Some(slot) = self.slots.get(allocation.index).copied() else {
            return Err(SlotManagerError::InvalidAllocation);
        };
        let mask = 1_u32 << allocation.index;
        if slot != allocation.slot || self.occupied & mask == 0 {
            return Err(SlotManagerError::InvalidAllocation);
        }
        self.occupied &= !mask;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dali_targets::IsolationSlot;

    const SLOT0_ID: u8 = 0;
    const SLOT1_ID: u8 = 1;
    const SLOT0_CODE_ORIGIN: u32 = 0x1000;
    const SLOT0_DATA_ORIGIN: u32 = 0x5000;
    const SLOT1_CODE_ORIGIN: u32 = 0x9000;
    const SLOT1_DATA_ORIGIN: u32 = 0xD000;
    const SLOT_CODE_LENGTH: u32 = 16 * 1024;
    const SLOT_DATA_LENGTH: u32 = 16 * 1024;
    const SLOT_STACK_LENGTH: u32 = 4 * 1024;
    const SLOT_TEMPLATE: IsolationSlot = IsolationSlot {
        id: SLOT0_ID,
        name: "fixture",
        code_origin: SLOT0_CODE_ORIGIN,
        code_length: SLOT_CODE_LENGTH,
        data_origin: SLOT0_DATA_ORIGIN,
        data_length: SLOT_DATA_LENGTH,
        stack_length: SLOT_STACK_LENGTH,
    };
    const SLOTS: &[IsolationSlot] = &[
        IsolationSlot {
            id: SLOT0_ID,
            name: "slot0",
            code_origin: SLOT0_CODE_ORIGIN,
            code_length: SLOT_CODE_LENGTH,
            data_origin: SLOT0_DATA_ORIGIN,
            data_length: SLOT_DATA_LENGTH,
            stack_length: SLOT_STACK_LENGTH,
        },
        IsolationSlot {
            id: SLOT1_ID,
            name: "slot1",
            code_origin: SLOT1_CODE_ORIGIN,
            code_length: SLOT_CODE_LENGTH,
            data_origin: SLOT1_DATA_ORIGIN,
            data_length: SLOT_DATA_LENGTH,
            stack_length: SLOT_STACK_LENGTH,
        },
    ];

    #[test]
    fn allocates_manifest_slots_in_order() {
        let mut manager = super::SlotManager::new(SLOTS).expect("capacity is sufficient");

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.allocate().expect("slot 0").index(), 0);
        assert_eq!(manager.allocate().expect("slot 1").index(), 1);
        assert!(manager.allocate().is_none());
    }

    #[test]
    fn releases_and_reuses_a_manifest_slot() {
        let mut manager = super::SlotManager::new(SLOTS).expect("capacity is sufficient");
        let allocation = manager.allocate().expect("slot 0");

        manager
            .release(allocation)
            .expect("active allocation releases");
        assert_eq!(manager.allocate().expect("slot 0 reuses").index(), 0);
    }

    #[test]
    fn reserves_only_the_declared_free_slot() {
        let mut manager = super::SlotManager::new(SLOTS).expect("capacity is sufficient");
        let slot = SLOTS[1];

        let allocation = manager.reserve(slot).expect("slot 1 is declared");
        assert_eq!(allocation.index(), 1);
        assert!(manager.is_reserved(slot));
        assert!(matches!(
            manager.reserve(slot),
            Err(super::SlotManagerError::SlotOccupied)
        ));
    }

    #[test]
    fn rejects_an_undeclared_slot() {
        let mut manager = super::SlotManager::new(SLOTS).expect("capacity is sufficient");
        let undeclared = IsolationSlot {
            name: "other",
            ..SLOTS[0]
        };

        assert!(matches!(
            manager.reserve(undeclared),
            Err(super::SlotManagerError::UndeclaredSlot)
        ));
    }

    #[test]
    fn confines_ranges_to_the_allocated_slot() {
        let mut manager = super::SlotManager::new(SLOTS).expect("capacity is sufficient");
        let slot0 = manager.allocate().expect("slot 0");
        let slot1 = manager.allocate().expect("slot 1");

        assert!(slot0.contains(super::SlotRegion::Code, SLOT0_CODE_ORIGIN, 4));
        assert!(slot0.contains(super::SlotRegion::Data, SLOT0_DATA_ORIGIN, 4));
        assert!(!slot0.contains(super::SlotRegion::Code, SLOT1_CODE_ORIGIN, 4));
        assert!(!slot0.contains(super::SlotRegion::Data, SLOT1_DATA_ORIGIN, 4));
        assert!(!slot1.contains(super::SlotRegion::Code, u32::MAX, 2));
    }

    #[test]
    fn rejects_capacity_shorter_than_the_manifest() {
        const OVER_CAPACITY: &[IsolationSlot] = &[SLOT_TEMPLATE; 33];
        assert!(matches!(
            super::SlotManager::new(OVER_CAPACITY),
            Err(super::SlotManagerError::CapacityExceeded)
        ));
    }
}
