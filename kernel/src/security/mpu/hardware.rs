//! Privileged ARMv7-M MPU register programming.

use super::IsolationLayout;
use super::descriptor::{MpuAccess, MpuExecution, MpuRegion};

const REGION_KERNEL: u32 = 0;
const REGION_RUNTIME: u32 = 1;
const REGION_APPLICATION_CODE: u32 = 2;
const REGION_APPLICATION_DATA: u32 = 3;
const REGION_PERIPHERALS: u32 = 4;
const REGION_BASE_MASK: u32 = !0x1F;
const MPU_CONTROL_ENABLE: u32 = 1;
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

const fn loader_region(region: MpuRegion) -> MpuRegion {
    MpuRegion {
        base: region.base,
        length: region.length,
        access: MpuAccess::PrivilegedOnly,
        execution: MpuExecution::Never,
        memory_type: region.memory_type,
    }
}

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
