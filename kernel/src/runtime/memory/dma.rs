//! DMA buffer ownership and target-range validation.

pub use dali_kernel_api::dma::{
    ApplicationDmaPolicy, CURRENT_POLICY, DmaBuffer, DmaBufferError, DmaOwner, DmaOwnershipError,
    DmaPolicy, validate_range,
};

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
