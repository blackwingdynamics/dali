//! Minimal SCB MMIO access used by kernel fault and test paths.

/// Memory-mapped base address of the System Control Block.
const SCB_BASE: usize = 0xE000_ED00;
#[cfg(feature = "abi-test-fixtures")]
/// SCB system-handler control and state register offset.
const SHCSR_OFFSET: usize = 0x24;
/// SCB configurable fault-status register offset.
const CFSR_OFFSET: usize = 0x28;
/// SCB hard-fault status register offset.
const HFSR_OFFSET: usize = 0x2C;
/// SCB memory-management fault address register offset.
const MMFAR_OFFSET: usize = 0x34;
/// SCB bus-fault address register offset.
const BFAR_OFFSET: usize = 0x38;

#[inline]
#[cfg(feature = "abi-test-fixtures")]
/// Reads the system-handler control and state register.
pub(crate) fn read_shcsr() -> u32 {
    read_register(SCB_BASE + SHCSR_OFFSET)
}

#[inline]
#[cfg(feature = "abi-test-fixtures")]
/// Writes the system-handler control and state register.
pub(crate) fn write_shcsr(value: u32) {
    write_register(SCB_BASE + SHCSR_OFFSET, value);
}

#[inline]
/// Reads the configurable fault-status register.
pub(crate) fn read_cfsr() -> u32 {
    read_register(SCB_BASE + CFSR_OFFSET)
}

#[inline]
/// Reads the hard-fault status register.
pub(crate) fn read_hfsr() -> u32 {
    read_register(SCB_BASE + HFSR_OFFSET)
}

#[inline]
/// Reads the memory-management fault address register.
pub(crate) fn read_mmfar() -> u32 {
    read_register(SCB_BASE + MMFAR_OFFSET)
}

#[inline]
/// Reads the bus-fault address register.
pub(crate) fn read_bfar() -> u32 {
    read_register(SCB_BASE + BFAR_OFFSET)
}

#[inline]
/// Performs a volatile read from an SCB register.
fn read_register(address: usize) -> u32 {
    unsafe {
        // SAFETY: These addresses are fixed, word-aligned ARMv7-M SCB registers.
        // Callers use this module only from privileged kernel paths. Inline
        // assembly avoids Rust pointer-validity checks while recovering faults.
        let value;
        core::arch::asm!(
            "ldr {value}, [{address}]",
            address = in(reg) address,
            value = out(reg) value,
            options(nostack, preserves_flags, readonly),
        );
        value
    }
}

#[inline]
#[cfg(feature = "abi-test-fixtures")]
fn write_register(address: usize, value: u32) {
    unsafe {
        // SAFETY: These addresses are fixed, word-aligned ARMv7-M SCB registers.
        // Callers use this module only from privileged kernel paths.
        core::arch::asm!(
            "str {value}, [{address}]",
            address = in(reg) address,
            value = in(reg) value,
            options(nostack, preserves_flags),
        );
    }
}
