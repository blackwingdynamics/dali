//! Manifest-driven application slot allocation.

use dali_targets::IsolationSlot;

const SLOT_MASK_BITS: usize = u32::BITS as usize;

/// A selected application slot and its stable manifest index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotAllocation {
    index: usize,
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
}

/// Errors returned while constructing or releasing a slot manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotManagerError {
    /// The caller-provided fixed capacity cannot hold the manifest slots.
    CapacityExceeded,
    /// The allocation was not active or did not match the manifest entry.
    InvalidAllocation,
}

/// Fixed-capacity allocator for manifest-declared application slots.
///
/// The manager owns only allocation state. Slot boundaries remain immutable
/// values supplied by the target manifest; no address arithmetic or filename
/// convention can create a slot.
pub struct SlotManager {
    slots: &'static [IsolationSlot],
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
    use super::{SlotManager, SlotManagerError};
    use dali_targets::IsolationSlot;

    const SLOT0_CODE_ORIGIN: u32 = 0x1000;
    const SLOT0_DATA_ORIGIN: u32 = 0x2000;
    const SLOT1_CODE_ORIGIN: u32 = 0x3000;
    const SLOT1_DATA_ORIGIN: u32 = 0x4000;
    const SLOT_CODE_LENGTH: u32 = 16 * 1024;
    const SLOT_DATA_LENGTH: u32 = 16 * 1024;
    const SLOT_STACK_LENGTH: u32 = 4 * 1024;
    const SLOT_TEMPLATE: IsolationSlot = IsolationSlot {
        name: "fixture",
        code_origin: SLOT0_CODE_ORIGIN,
        code_length: SLOT_CODE_LENGTH,
        data_origin: SLOT0_DATA_ORIGIN,
        data_length: SLOT_DATA_LENGTH,
        stack_length: SLOT_STACK_LENGTH,
    };
    const SLOTS: &[IsolationSlot] = &[
        IsolationSlot {
            name: "slot0",
            code_origin: SLOT0_CODE_ORIGIN,
            code_length: SLOT_CODE_LENGTH,
            data_origin: SLOT0_DATA_ORIGIN,
            data_length: SLOT_DATA_LENGTH,
            stack_length: SLOT_STACK_LENGTH,
        },
        IsolationSlot {
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
        let mut manager = SlotManager::new(SLOTS).expect("capacity is sufficient");

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.allocate().expect("slot 0").index(), 0);
        assert_eq!(manager.allocate().expect("slot 1").index(), 1);
        assert!(manager.allocate().is_none());
    }

    #[test]
    fn releases_and_reuses_a_manifest_slot() {
        let mut manager = SlotManager::new(SLOTS).expect("capacity is sufficient");
        let allocation = manager.allocate().expect("slot 0");

        manager
            .release(allocation)
            .expect("active allocation releases");
        assert_eq!(manager.allocate().expect("slot 0 reuses").index(), 0);
    }

    #[test]
    fn rejects_capacity_shorter_than_the_manifest() {
        const OVER_CAPACITY: &[IsolationSlot] = &[SLOT_TEMPLATE; 33];
        assert!(matches!(
            SlotManager::new(OVER_CAPACITY),
            Err(SlotManagerError::CapacityExceeded)
        ));
    }
}
