//! Manifest-driven memory layout construction for application isolation.

use dali_targets::MemoryProfile;

use super::descriptor::{MemoryRegion, MpuAccess, MpuExecution, MpuMemoryType, MpuRegion};

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
        let isolation = match memory.isolation {
            Some(isolation) => isolation,
            None => return None,
        };
        let active_slot = match isolation.active_slot() {
            Some(slot) => slot,
            None => return None,
        };
        Self::from_memory_for_slot(memory, active_slot)
    }

    /// Builds a layout using one slot from the board manifest's slot table.
    pub const fn from_memory_for_slot(
        memory: MemoryProfile,
        active_slot: dali_targets::IsolationSlot,
    ) -> Option<Self> {
        let isolation = match memory.isolation {
            Some(isolation) => isolation,
            None => return None,
        };
        let kernel = match MpuRegion::new(
            memory.kernel_origin,
            memory.kernel_length,
            MpuAccess::PrivilegedOnly,
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
        let application_code = match MpuRegion::new(
            active_slot.code_origin,
            active_slot.code_length,
            MpuAccess::ReadOnly,
            MpuExecution::Allowed,
        ) {
            Some(region) => region,
            None => return None,
        };
        let application_data = match MpuRegion::new(
            active_slot.data_origin,
            active_slot.data_length,
            MpuAccess::ReadWrite,
            MpuExecution::Never,
        ) {
            Some(region) => region,
            None => return None,
        };
        let runtime_memory = match memory.ccm {
            Some(region) => region,
            None => return None,
        };
        let runtime = match MpuRegion::new(
            runtime_memory.origin,
            runtime_memory.length,
            MpuAccess::PrivilegedOnly,
            MpuExecution::Never,
        ) {
            Some(region) => region,
            None => return None,
        };
        let peripherals = match (isolation.peripheral_origin, isolation.peripheral_length) {
            (Some(origin), Some(length)) => match MpuRegion::with_memory_type(
                origin,
                length,
                MpuAccess::PrivilegedOnly,
                MpuExecution::Never,
                MpuMemoryType::Device,
            ) {
                Some(region) => region,
                None => return None,
            },
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
