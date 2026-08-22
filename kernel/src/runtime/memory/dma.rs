//! DMA buffer ownership and target-range validation.

use core::mem::align_of;

use dali_targets::TargetMemoryRegion;

/// Owner permitted to configure a DMA transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaOwner {
    /// A kernel-owned peripheral transport.
    Kernel,
    /// An application request crossing the kernel DMA boundary.
    Application,
}

/// Application DMA policy for the current single-application contract.
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

/// Kernel-owned DMA authorization policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaPolicy {
    application: ApplicationDmaPolicy,
}

impl DmaPolicy {
    /// Returns the policy for the current application execution contract.
    pub const fn current() -> Self {
        Self {
            application: ApplicationDmaPolicy::Denied,
        }
    }

    /// Authorizes one owner before any peripheral DMA configuration.
    pub const fn authorize(self, owner: DmaOwner) -> Result<(), DmaOwnershipError> {
        match (owner, self.application) {
            (DmaOwner::Kernel, _) => Ok(()),
            (DmaOwner::Application, ApplicationDmaPolicy::Denied) => {
                Err(DmaOwnershipError::ApplicationDmaDenied)
            }
        }
    }
}

/// Single source of truth for application DMA ownership.
pub const CURRENT_POLICY: DmaPolicy = DmaPolicy::current();

/// Rejects a buffer before a peripheral DMA controller can be programmed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaBufferError {
    /// The buffer does not fit completely inside the target's DMA region.
    OutsideDmaRegion,
    /// The buffer address is not aligned for its element type.
    Misaligned,
    /// The buffer has no transferable elements.
    Empty,
    /// The byte-range calculation overflowed.
    RangeOverflow,
    /// The address cannot be represented by the target DMA register width.
    AddressOverflow,
}

/// A mutable buffer whose address and extent were validated for DMA use.
pub struct DmaBuffer<'a, T> {
    buffer: &'a mut [T],
}

impl<'a, T> DmaBuffer<'a, T> {
    /// Validates a caller-owned buffer against the target-declared DMA region.
    pub fn new(buffer: &'a mut [T], region: TargetMemoryRegion) -> Result<Self, DmaBufferError> {
        validate_buffer(buffer, region)?;
        Ok(Self { buffer })
    }

    /// Returns the exclusively borrowed buffer for one active DMA transfer.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.buffer
    }
}

fn validate_buffer<T>(buffer: &[T], region: TargetMemoryRegion) -> Result<(), DmaBufferError> {
    let start =
        u32::try_from(buffer.as_ptr() as usize).map_err(|_| DmaBufferError::AddressOverflow)?;
    let length = buffer
        .len()
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(DmaBufferError::RangeOverflow)?;
    let length = u32::try_from(length).map_err(|_| DmaBufferError::RangeOverflow)?;
    validate_range(start, length, align_of::<T>(), region)
}

/// Checks an address range without touching hardware or fabricating a device.
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

#[cfg(test)]
mod tests {
    use dali_targets::TargetMemoryRegion;

    const DMA_REGION: TargetMemoryRegion = TargetMemoryRegion {
        origin: 0x2000_0000,
        length: 0x8000,
    };

    #[test]
    fn accepts_aligned_range_inside_declared_region() {
        assert_eq!(
            super::validate_range(0x2000_0100, 0x100, 4, DMA_REGION),
            Ok(())
        );
    }

    #[test]
    fn rejects_range_outside_declared_region() {
        assert_eq!(
            super::validate_range(0x2000_7F00, 0x200, 4, DMA_REGION),
            Err(super::DmaBufferError::OutsideDmaRegion)
        );
    }

    #[test]
    fn rejects_unaligned_and_empty_ranges() {
        assert_eq!(
            super::validate_range(0x2000_0002, 4, 4, DMA_REGION),
            Err(super::DmaBufferError::Misaligned)
        );
        assert_eq!(
            super::validate_range(0x2000_0000, 0, 4, DMA_REGION),
            Err(super::DmaBufferError::Empty)
        );
    }

    #[test]
    fn rejects_overflowing_ranges() {
        assert_eq!(
            super::validate_range(u32::MAX - 3, 8, 4, DMA_REGION),
            Err(super::DmaBufferError::RangeOverflow)
        );
    }

    #[test]
    fn rejects_application_dma_configuration() {
        assert_eq!(
            super::CURRENT_POLICY.authorize(super::DmaOwner::Application),
            Err(super::DmaOwnershipError::ApplicationDmaDenied)
        );
    }

    #[test]
    fn authorizes_kernel_transport_dma() {
        assert_eq!(
            super::CURRENT_POLICY.authorize(super::DmaOwner::Kernel),
            Ok(())
        );
    }
}
