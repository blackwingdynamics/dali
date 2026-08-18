use super::IsolationLayout;
use super::descriptor::{
    AP_READ_ONLY, AP_READ_WRITE, MINIMUM_REGION_BYTES, MpuAccess, MpuExecution, MpuMemoryType,
    MpuRegion, RASR_ENABLE, RASR_XN,
};

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
    let layout = IsolationLayout::from_memory(dali_targets::TARGET_F405.memory);
    assert!(layout.is_some());
    let Some(layout) = layout else {
        return;
    };
    assert_eq!(
        layout.kernel.base,
        dali_targets::TARGET_F405.memory.kernel_origin
    );
    assert_eq!(
        layout.application.length,
        dali_targets::TARGET_F405.memory.application_length
    );
    assert_eq!(layout.kernel.access, MpuAccess::PrivilegedOnly);
    assert!(layout.application_single_region.is_none());
    assert_eq!(layout.application_code.access, MpuAccess::ReadOnly);
    assert_eq!(layout.application_data.execution, MpuExecution::Never);
    assert_eq!(layout.peripherals.access, MpuAccess::PrivilegedOnly);
    assert_eq!(layout.peripherals.memory_type, MpuMemoryType::Device);
}

#[test]
fn builds_application_regions_for_a_declared_nonzero_slot() {
    let Some(isolation) = dali_targets::TARGET_F405.memory.isolation else {
        return;
    };
    let Some(slot) = isolation.slots.get(1).copied() else {
        return;
    };
    let Some(layout) =
        IsolationLayout::from_memory_for_slot(dali_targets::TARGET_F405.memory, slot)
    else {
        return;
    };
    assert_eq!(layout.application_code.base, slot.code_origin);
    assert_eq!(layout.application_data.base, slot.data_origin);
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
