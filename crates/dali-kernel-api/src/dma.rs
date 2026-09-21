//! Hardware-neutral DMA ownership and buffer validation contracts.

use core::mem::align_of;
use dali_targets::TargetMemoryRegion;

/// Owner permitted to configure a DMA transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaOwner {
    /// A board-owned kernel transport.
    Kernel,
    /// An application request crossing the kernel DMA boundary.
    Application,
}

/// Application DMA policy for the current execution contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationDmaPolicy {
    /// Applications have no DMA service or peripheral ownership.
    Denied,
}

/// Errors returned when a caller requests DMA ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaOwnershipError {
    /// The current policy does not expose DMA to applications.
    ApplicationDmaDenied,
}

/// Bounded DMA authorization policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaPolicy {
    application: ApplicationDmaPolicy,
}

impl DmaPolicy {
    /// Returns the policy for the current single-application contract.
    pub const fn current() -> Self {
        Self {
            application: ApplicationDmaPolicy::Denied,
        }
    }

    /// Authorizes one owner before peripheral configuration.
    pub const fn authorize(self, owner: DmaOwner) -> Result<(), DmaOwnershipError> {
        match (owner, self.application) {
            (DmaOwner::Kernel, _) => Ok(()),
            (DmaOwner::Application, ApplicationDmaPolicy::Denied) => {
                Err(DmaOwnershipError::ApplicationDmaDenied)
            }
        }
    }
}

/// Single source of truth for the current DMA ownership policy.
pub const CURRENT_POLICY: DmaPolicy = DmaPolicy::current();

/// Errors returned when validating a DMA buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaBufferError {
    /// The buffer does not fit inside the declared DMA region.
    OutsideDmaRegion,
    /// The buffer address is not aligned for its element type.
    Misaligned,
    /// The buffer has no transferable elements.
    Empty,
    /// The byte-range calculation overflowed.
    RangeOverflow,
    /// The address cannot be represented by the target register width.
    AddressOverflow,
}

/// Mutable buffer whose address and extent were validated for DMA use.
pub struct DmaBuffer<'a, T> {
    buffer: &'a mut [T],
}

impl<'a, T> DmaBuffer<'a, T> {
    /// Validates a caller-owned buffer against a target-declared DMA region.
    pub fn new(buffer: &'a mut [T], region: TargetMemoryRegion) -> Result<Self, DmaBufferError> {
        let start =
            u32::try_from(buffer.as_ptr() as usize).map_err(|_| DmaBufferError::AddressOverflow)?;
        let length = buffer
            .len()
            .checked_mul(core::mem::size_of::<T>())
            .ok_or(DmaBufferError::RangeOverflow)?;
        let length = u32::try_from(length).map_err(|_| DmaBufferError::RangeOverflow)?;
        validate_range(start, length, align_of::<T>(), region)?;
        Ok(Self { buffer })
    }

    /// Returns the exclusively borrowed buffer for one active transfer.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.buffer
    }
}

/// Validates an address range without touching hardware.
pub fn validate_range(
    start: u32,
    length: u32,
    alignment: usize,
    region: TargetMemoryRegion,
) -> Result<(), DmaBufferError> {
    if length == 0 {
        return Err(DmaBufferError::Empty);
    }
    if alignment == 0 || !(start as usize).is_multiple_of(alignment) {
        return Err(DmaBufferError::Misaligned);
    }
    let end = start
        .checked_add(length)
        .ok_or(DmaBufferError::RangeOverflow)?;
    let region_end = region
        .origin
        .checked_add(region.length)
        .ok_or(DmaBufferError::RangeOverflow)?;
    if start < region.origin || end > region_end {
        return Err(DmaBufferError::OutsideDmaRegion);
    }
    Ok(())
}
