//! Board-owned memory protection descriptors.
//!
//! This module describes the first F405 isolation layout without enabling the
//! MPU. Hardware activation is intentionally deferred until the SVC, PSP, and
//! fault-recovery contracts are implemented together.

use dali_targets::MemoryProfile;

/// Smallest region size supported by the ARMv7-M MPU.
const MINIMUM_REGION_BYTES: u32 = 32;

/// Access permitted to an application in an MPU region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpuAccess {
    /// All application accesses are rejected.
    NoAccess,
    /// Application reads are allowed, but writes are rejected.
    ReadOnly,
    /// Application reads and writes are allowed.
    ReadWrite,
}

/// Whether application instruction fetches are permitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpuExecution {
    /// Instruction fetches are allowed.
    Allowed,
    /// Instruction fetches are rejected.
    Never,
}

/// A manifest-declared memory boundary that may require multiple MPU regions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    /// Region base address.
    pub base: u32,
    /// Region size in bytes.
    pub length: u32,
}

/// A validated ARMv7-M MPU region descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MpuRegion {
    /// Region base address.
    pub base: u32,
    /// Region size in bytes.
    pub length: u32,
    /// Application access permission.
    pub access: MpuAccess,
    /// Application instruction-fetch permission.
    pub execution: MpuExecution,
}

impl MpuRegion {
    /// Creates a descriptor when the ARMv7-M alignment rules are satisfied.
    pub const fn new(
        base: u32,
        length: u32,
        access: MpuAccess,
        execution: MpuExecution,
    ) -> Option<Self> {
        if length < MINIMUM_REGION_BYTES
            || (length & (length - 1)) != 0
            || !base.is_multiple_of(length)
        {
            return None;
        }

        Some(Self {
            base,
            length,
            access,
            execution,
        })
    }
}

/// Memory descriptors used by the first single-application isolation mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsolationLayout {
    /// Kernel memory, inaccessible to the application.
    pub kernel: MpuRegion,
    /// Application memory boundary, pending code/data region decomposition.
    pub application: MemoryRegion,
    /// Single-region mapping when the manifest boundary satisfies MPU rules.
    pub application_single_region: Option<MpuRegion>,
    /// Application code region with read/execute permissions.
    pub application_code: MpuRegion,
    /// Application data and PSP region with read/write, execute-never permissions.
    pub application_data: MpuRegion,
    /// Kernel runtime and stack memory, inaccessible to the application.
    pub runtime: MpuRegion,
    /// Ordinary peripheral registers, inaccessible to the application.
    pub peripherals: MpuRegion,
}

impl IsolationLayout {
    /// Builds a layout from the board manifest's declared SRAM regions.
    pub const fn from_memory(memory: MemoryProfile) -> Option<Self> {
        let kernel = match MpuRegion::new(
            memory.kernel_origin,
            memory.kernel_length,
            MpuAccess::NoAccess,
            MpuExecution::Never,
        ) {
            Some(region) => region,
            None => return None,
        };
        let application = MemoryRegion {
            base: memory.application_origin,
            length: memory.application_length,
        };
        let application_single_region = MpuRegion::new(
            memory.application_origin,
            memory.application_length,
            MpuAccess::ReadWrite,
            MpuExecution::Allowed,
        );
        let isolation = match memory.isolation {
            Some(isolation) => isolation,
            None => return None,
        };
        let application_code = match MpuRegion::new(
            isolation.code_origin,
            isolation.code_length,
            MpuAccess::ReadOnly,
            MpuExecution::Allowed,
        ) {
            Some(region) => region,
            None => return None,
        };
        let application_data = match MpuRegion::new(
            isolation.data_origin,
            isolation.data_length,
            MpuAccess::ReadWrite,
            MpuExecution::Never,
        ) {
            Some(region) => region,
            None => return None,
        };
        let runtime = match MpuRegion::new(
            memory.runtime_origin,
            memory.runtime_length,
            MpuAccess::NoAccess,
            MpuExecution::Never,
        ) {
            Some(region) => region,
            None => return None,
        };
        let peripherals = match (isolation.peripheral_origin, isolation.peripheral_length) {
            (Some(origin), Some(length)) => {
                match MpuRegion::new(origin, length, MpuAccess::NoAccess, MpuExecution::Never) {
                    Some(region) => region,
                    None => return None,
                }
            }
            _ => return None,
        };

        Some(Self {
            kernel,
            application,
            application_single_region,
            application_code,
            application_data,
            runtime,
            peripherals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{IsolationLayout, MINIMUM_REGION_BYTES, MpuAccess, MpuExecution, MpuRegion};
    use dali_targets::TARGET_F405;

    const MISALIGNED_BASE: u32 = 1;
    const NON_POWER_OF_TWO_BYTES: u32 = 48;

    #[test]
    fn rejects_non_power_of_two_or_misaligned_regions() {
        assert!(
            MpuRegion::new(
                MISALIGNED_BASE,
                MINIMUM_REGION_BYTES,
                MpuAccess::NoAccess,
                MpuExecution::Never
            )
            .is_none()
        );
        assert!(
            MpuRegion::new(
                MINIMUM_REGION_BYTES,
                NON_POWER_OF_TWO_BYTES,
                MpuAccess::NoAccess,
                MpuExecution::Never
            )
            .is_none()
        );
    }

    #[test]
    fn describes_the_manifest_memory_boundaries() {
        let layout = IsolationLayout::from_memory(TARGET_F405.memory);
        assert!(layout.is_some());
        let Some(layout) = layout else {
            return;
        };
        assert_eq!(layout.kernel.base, TARGET_F405.memory.kernel_origin);
        assert_eq!(
            layout.application.length,
            TARGET_F405.memory.application_length
        );
        assert_eq!(layout.kernel.access, MpuAccess::NoAccess);
        assert!(layout.application_single_region.is_none());
        assert_eq!(layout.application_code.access, MpuAccess::ReadOnly);
        assert_eq!(layout.application_data.execution, MpuExecution::Never);
        assert_eq!(layout.peripherals.access, MpuAccess::NoAccess);
    }
}
