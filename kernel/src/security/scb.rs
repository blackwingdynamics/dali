//! Minimal SCB MMIO access used by kernel fault and test paths.

const SCB_BASE: usize = 0xE000_ED00;
const SHCSR_OFFSET: usize = 0x24;
const CFSR_OFFSET: usize = 0x28;
const MMFAR_OFFSET: usize = 0x34;
const BFAR_OFFSET: usize = 0x38;

#[inline]
pub(crate) fn read_shcsr() -> u32 {
    read_register(SCB_BASE + SHCSR_OFFSET)
}

#[inline]
pub(crate) fn write_shcsr(value: u32) {
    write_register(SCB_BASE + SHCSR_OFFSET, value);
}

#[inline]
pub(crate) fn read_cfsr() -> u32 {
    read_register(SCB_BASE + CFSR_OFFSET)
}

#[inline]
pub(crate) fn read_mmfar() -> u32 {
    read_register(SCB_BASE + MMFAR_OFFSET)
}

#[inline]
pub(crate) fn read_bfar() -> u32 {
    read_register(SCB_BASE + BFAR_OFFSET)
}

#[inline]
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
