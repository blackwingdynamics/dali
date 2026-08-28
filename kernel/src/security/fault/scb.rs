//! Minimal SCB MMIO access used by kernel fault and test paths.

use dali_kernel_api::FaultRegister;

#[inline]
#[cfg(feature = "abi-test-fixtures")]
/// Reads the system-handler control and state register.
pub(crate) fn read_shcsr() -> u32 {
    crate::platform::read_fault_register(FaultRegister::SystemHandlerControl)
}

#[inline]
#[cfg(feature = "abi-test-fixtures")]
/// Writes the system-handler control and state register.
pub(crate) fn write_shcsr(value: u32) {
    crate::platform::write_fault_register(FaultRegister::SystemHandlerControl, value);
}

#[inline]
/// Reads the configurable fault-status register.
pub(crate) fn read_cfsr() -> u32 {
    crate::platform::read_fault_register(FaultRegister::Configurable)
}

#[inline]
/// Reads the hard-fault status register.
pub(crate) fn read_hfsr() -> u32 {
    crate::platform::read_fault_register(FaultRegister::Hard)
}

#[inline]
/// Reads the memory-management fault address register.
pub(crate) fn read_mmfar() -> u32 {
    crate::platform::read_fault_register(FaultRegister::MemoryAddress)
}

#[inline]
/// Reads the bus-fault address register.
pub(crate) fn read_bfar() -> u32 {
    crate::platform::read_fault_register(FaultRegister::BusAddress)
}
