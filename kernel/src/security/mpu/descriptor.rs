//! ARMv7-M MPU region descriptors and attribute encoding.

/// Smallest region size supported by the ARMv7-M MPU.
pub(crate) const MINIMUM_REGION_BYTES: u32 = 32;
/// Defines the `RASR_ENABLE` bound used by this subsystem.
pub(crate) const RASR_ENABLE: u32 = 1;
/// Defines the `RASR_SIZE_SHIFT` bound used by this subsystem.
const RASR_SIZE_SHIFT: u32 = 1;
/// Defines the `RASR_AP_SHIFT` bound used by this subsystem.
const RASR_AP_SHIFT: u32 = 24;
/// Defines the `RASR_XN` bound used by this subsystem.
pub(crate) const RASR_XN: u32 = 1 << 28;
/// Defines the `RASR_SHAREABLE` bound used by this subsystem.
const RASR_SHAREABLE: u32 = 1 << 18;
/// Defines the `RASR_CACHEABLE` bound used by this subsystem.
const RASR_CACHEABLE: u32 = 1 << 17;
/// Defines the `RASR_BUFFERABLE` bound used by this subsystem.
const RASR_BUFFERABLE: u32 = 1 << 16;
/// Defines the `AP_PRIVILEGED_ONLY` bound used by this subsystem.
const AP_PRIVILEGED_ONLY: u32 = 0b001 << RASR_AP_SHIFT;
/// Defines the `AP_READ_ONLY` bound used by this subsystem.
pub(crate) const AP_READ_ONLY: u32 = 0b110 << RASR_AP_SHIFT;
/// Defines the `AP_READ_WRITE` bound used by this subsystem.
pub(crate) const AP_READ_WRITE: u32 = 0b011 << RASR_AP_SHIFT;

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
