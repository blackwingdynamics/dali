//! Privileged ARMv7-M MPU register programming.

use super::IsolationLayout;
use super::descriptor::{MpuAccess, MpuExecution, MpuRegion};

/// MPU region number reserved for kernel memory.
const REGION_KERNEL: u32 = 0;
/// MPU region number reserved for kernel runtime memory.
const REGION_RUNTIME: u32 = 1;
/// MPU region number reserved for application code.
const REGION_APPLICATION_CODE: u32 = 2;
/// MPU region number reserved for application data and stack.
const REGION_APPLICATION_DATA: u32 = 3;
/// MPU region number reserved for peripheral access policy.
const REGION_PERIPHERALS: u32 = 4;
/// Mask retaining the aligned base address bits required by the MPU.
const REGION_BASE_MASK: u32 = !0x1F;
/// MPU control bit that enables the unit.
const MPU_CONTROL_ENABLE: u32 = 1;
/// MPU control bit that enables the privileged default map.
const MPU_CONTROL_PRIVILEGED_DEFAULT: u32 = 1 << 2;

/// Programs the planned single-application MPU map.
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

/// Enables the fault handlers required by the MPU recovery boundary.
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
pub fn activate_application_regions(layout: IsolationLayout) {
    let regions = [
        (REGION_APPLICATION_CODE, layout.application_code),
        (REGION_APPLICATION_DATA, layout.application_data),
    ];
    write_regions(&regions, true);
}

/// Creates the loader-time privileged-only form of an application region.
const fn loader_region(region: MpuRegion) -> MpuRegion {
    MpuRegion {
        base: region.base,
        length: region.length,
        access: MpuAccess::PrivilegedOnly,
        execution: MpuExecution::Never,
        memory_type: region.memory_type,
    }
}

/// Writes a bounded set of MPU regions and optionally enables the resulting map.
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
