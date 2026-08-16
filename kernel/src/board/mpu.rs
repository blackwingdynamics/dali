//! Board-owned memory protection descriptors.
//!
//! This module owns the first F405 isolation layout and its privileged MPU
//! programming sequence.

use dali_targets::MemoryProfile;

/// Smallest region size supported by the ARMv7-M MPU.
const MINIMUM_REGION_BYTES: u32 = 32;
const RASR_ENABLE: u32 = 1;
const RASR_SIZE_SHIFT: u32 = 1;
const RASR_AP_SHIFT: u32 = 24;
const RASR_XN: u32 = 1 << 28;
const RASR_SHAREABLE: u32 = 1 << 18;
const RASR_CACHEABLE: u32 = 1 << 17;
const RASR_BUFFERABLE: u32 = 1 << 16;
const AP_PRIVILEGED_ONLY: u32 = 0b001 << RASR_AP_SHIFT;
const AP_READ_ONLY: u32 = 0b110 << RASR_AP_SHIFT;
const AP_READ_WRITE: u32 = 0b011 << RASR_AP_SHIFT;
#[cfg(feature = "abi-v3-mpu")]
const REGION_KERNEL: u32 = 0;
#[cfg(feature = "abi-v3-mpu")]
const REGION_RUNTIME: u32 = 1;
#[cfg(feature = "abi-v3-mpu")]
const REGION_APPLICATION_CODE: u32 = 2;
#[cfg(feature = "abi-v3-mpu")]
const REGION_APPLICATION_DATA: u32 = 3;
#[cfg(feature = "abi-v3-mpu")]
const REGION_PERIPHERALS: u32 = 4;
#[cfg(feature = "abi-v3-mpu")]
const REGION_BASE_MASK: u32 = !0x1F;
#[cfg(feature = "abi-v3-mpu")]
const MPU_CONTROL_ENABLE: u32 = 1;
#[cfg(feature = "abi-v3-mpu")]
const MPU_CONTROL_PRIVILEGED_DEFAULT: u32 = 1 << 2;

/// Access permitted to an application in an MPU region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpuAccess {
    /// Privileged kernel accesses are allowed; unprivileged accesses are rejected.
    PrivilegedOnly,
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

/// Memory type encoded in ARMv7-M MPU attributes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MpuMemoryType {
    /// Normal SRAM with cacheable, bufferable attributes.
    Normal,
    /// Shareable device registers used by ordinary peripherals.
    Device,
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
    /// Memory type encoded in the region attributes.
    pub memory_type: MpuMemoryType,
}

impl MpuRegion {
    /// Creates a descriptor when the ARMv7-M alignment rules are satisfied.
    pub const fn new(
        base: u32,
        length: u32,
        access: MpuAccess,
        execution: MpuExecution,
    ) -> Option<Self> {
        Self::with_memory_type(base, length, access, execution, MpuMemoryType::Normal)
    }

    /// Creates a descriptor with an explicit ARMv7-M memory type.
    pub const fn with_memory_type(
        base: u32,
        length: u32,
        access: MpuAccess,
        execution: MpuExecution,
        memory_type: MpuMemoryType,
    ) -> Option<Self> {
        if length < MINIMUM_REGION_BYTES
            || (length & (length - 1)) != 0
            || !base.is_multiple_of(length)
        {
            return None;
        }

        let region = Self {
            base,
            length,
            access,
            execution,
            memory_type,
        };
        let _ = region.rasr_bits();
        Some(region)
    }

    /// Encodes this descriptor as an ARMv7-M RASR value.
    pub const fn rasr_bits(self) -> u32 {
        let size = self.length.trailing_zeros() - 1;
        let access = match self.access {
            MpuAccess::PrivilegedOnly => AP_PRIVILEGED_ONLY,
            MpuAccess::ReadOnly => AP_READ_ONLY,
            MpuAccess::ReadWrite => AP_READ_WRITE,
        };
        let memory_type = match self.memory_type {
            MpuMemoryType::Normal => RASR_CACHEABLE | RASR_BUFFERABLE,
            MpuMemoryType::Device => RASR_SHAREABLE | RASR_BUFFERABLE,
        };
        let execution = match self.execution {
            MpuExecution::Allowed => 0,
            MpuExecution::Never => RASR_XN,
        };

        RASR_ENABLE | (size << RASR_SIZE_SHIFT) | memory_type | access | execution
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
            (Some(origin), Some(length)) => {
                match MpuRegion::with_memory_type(
                    origin,
                    length,
                    MpuAccess::PrivilegedOnly,
                    MpuExecution::Never,
                    MpuMemoryType::Device,
                ) {
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

/// Programs the planned single-application MPU map.
#[cfg(feature = "abi-v3-mpu")]
pub fn configure_hardware(layout: IsolationLayout) {
    enable_fault_handlers();
    let application_code = loader_region(layout.application_code);
    let application_data = loader_region(layout.application_data);
    let regions = [
        (REGION_KERNEL, layout.kernel),
        (REGION_RUNTIME, layout.runtime),
        (REGION_APPLICATION_CODE, application_code),
        (REGION_APPLICATION_DATA, application_data),
        (REGION_PERIPHERALS, layout.peripherals),
    ];
    write_regions(&regions, false);
}

#[cfg(feature = "abi-v3-mpu")]
fn enable_fault_handlers() {
    let mut scb = unsafe {
        // SAFETY: Bootstrap runs in privileged reset context before the
        // application can execute; no other code accesses this SCB token.
        cortex_m::Peripherals::steal().SCB
    };
    scb.enable(cortex_m::peripheral::scb::Exception::MemoryManagement);
    scb.enable(cortex_m::peripheral::scb::Exception::BusFault);
    scb.enable(cortex_m::peripheral::scb::Exception::UsageFault);
}

/// Changes application regions from loader permissions to user permissions.
#[cfg(feature = "abi-v3-mpu")]
pub fn activate_application_regions(layout: IsolationLayout) {
    let regions = [
        (REGION_APPLICATION_CODE, layout.application_code),
        (REGION_APPLICATION_DATA, layout.application_data),
    ];
    write_regions(&regions, true);
}

#[cfg(feature = "abi-v3-mpu")]
const fn loader_region(region: MpuRegion) -> MpuRegion {
    MpuRegion {
        base: region.base,
        length: region.length,
        access: MpuAccess::PrivilegedOnly,
        execution: MpuExecution::Never,
        memory_type: region.memory_type,
    }
}

#[cfg(feature = "abi-v3-mpu")]
fn write_regions(regions: &[(u32, MpuRegion)], enable: bool) {
    let mpu = unsafe {
        // SAFETY: The Cortex-M4 MPU is a unique architectural peripheral and
        // these functions are called by privileged kernel code only.
        &*cortex_m::peripheral::MPU::PTR
    };
    unsafe {
        // SAFETY: `mpu` points to the unique Cortex-M4 MPU register block and
        // all writes are performed by privileged kernel code.
        if !enable {
            mpu.ctrl.write(0);
        }
        for &(number, region) in regions {
            mpu.rnr.write(number);
            mpu.rbar.write(region.base & REGION_BASE_MASK);
            mpu.rasr.write(region.rasr_bits());
        }
        if !enable {
            mpu.ctrl
                .write(MPU_CONTROL_ENABLE | MPU_CONTROL_PRIVILEGED_DEFAULT);
        }
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

#[cfg(test)]
mod tests {
    use super::{
        AP_READ_ONLY, AP_READ_WRITE, IsolationLayout, MINIMUM_REGION_BYTES, MpuAccess,
        MpuExecution, MpuMemoryType, MpuRegion, RASR_ENABLE, RASR_XN,
    };
    use dali_targets::TARGET_F405;

    const MISALIGNED_BASE: u32 = 1;
    const NON_POWER_OF_TWO_BYTES: u32 = 48;

    #[test]
    fn rejects_non_power_of_two_or_misaligned_regions() {
        assert!(
            MpuRegion::new(
                MISALIGNED_BASE,
                MINIMUM_REGION_BYTES,
                MpuAccess::PrivilegedOnly,
                MpuExecution::Never
            )
            .is_none()
        );
        assert!(
            MpuRegion::new(
                MINIMUM_REGION_BYTES,
                NON_POWER_OF_TWO_BYTES,
                MpuAccess::PrivilegedOnly,
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
        assert_eq!(layout.kernel.access, MpuAccess::PrivilegedOnly);
        assert!(layout.application_single_region.is_none());
        assert_eq!(layout.application_code.access, MpuAccess::ReadOnly);
        assert_eq!(layout.application_data.execution, MpuExecution::Never);
        assert_eq!(layout.peripherals.access, MpuAccess::PrivilegedOnly);
        assert_eq!(layout.peripherals.memory_type, MpuMemoryType::Device);
    }

    #[test]
    fn encodes_application_permissions_and_execution_policy() {
        let Some(code) = MpuRegion::new(
            0x2000_8000,
            32_768,
            MpuAccess::ReadOnly,
            MpuExecution::Allowed,
        ) else {
            return;
        };
        let Some(data) = MpuRegion::new(
            0x2001_0000,
            32_768,
            MpuAccess::ReadWrite,
            MpuExecution::Never,
        ) else {
            return;
        };

        assert_eq!(code.rasr_bits() & AP_READ_ONLY, AP_READ_ONLY);
        assert_eq!(code.rasr_bits() & RASR_XN, 0);
        assert_eq!(code.rasr_bits() & RASR_ENABLE, RASR_ENABLE);
        assert_eq!(data.rasr_bits() & AP_READ_WRITE, AP_READ_WRITE);
        assert_eq!(data.rasr_bits() & RASR_XN, RASR_XN);
    }
}
