#[test]
fn rejects_non_power_of_two_or_misaligned_regions() {
    const MISALIGNED_BASE: u32 = 1;
    const NON_POWER_OF_TWO_BYTES: u32 = 48;
    assert!(
        super::descriptor::MpuRegion::new(
            MISALIGNED_BASE,
            super::descriptor::MINIMUM_REGION_BYTES,
            super::descriptor::MpuAccess::PrivilegedOnly,
            super::descriptor::MpuExecution::Never
        )
        .is_none()
    );
    assert!(
        super::descriptor::MpuRegion::new(
            super::descriptor::MINIMUM_REGION_BYTES,
            NON_POWER_OF_TWO_BYTES,
            super::descriptor::MpuAccess::PrivilegedOnly,
            super::descriptor::MpuExecution::Never
        )
        .is_none()
    );
}

#[test]
fn describes_the_manifest_memory_boundaries() {
    let target = dali_targets::SUPPORTED_TARGETS[0];
    let layout = super::IsolationLayout::from_memory(target.memory);
    assert!(layout.is_some());
    let Some(layout) = layout else {
        return;
    };
    assert_eq!(layout.kernel.base, target.memory.kernel_origin);
    assert_eq!(layout.application.length, target.memory.application_length);
    assert_eq!(
        layout.kernel.access,
        super::descriptor::MpuAccess::PrivilegedOnly
    );
    assert!(layout.application_single_region.is_none());
    assert_eq!(
        layout.application_code.access,
        super::descriptor::MpuAccess::ReadOnly
    );
    assert_eq!(
        layout.application_data.execution,
        super::descriptor::MpuExecution::Never
    );
    assert_eq!(
        layout.peripherals.access,
        super::descriptor::MpuAccess::PrivilegedOnly
    );
    assert_eq!(
        layout.peripherals.memory_type,
        super::descriptor::MpuMemoryType::Device
    );
}

#[test]
fn builds_application_regions_for_a_declared_nonzero_slot() {
    let target = dali_targets::SUPPORTED_TARGETS[0];
    let Some(isolation) = target.memory.isolation else {
        return;
    };
    let Some(slot) = isolation.slots.get(1).copied() else {
        return;
    };
    let Some(layout) = super::IsolationLayout::from_memory_for_slot(target.memory, slot) else {
        return;
    };
    assert_eq!(layout.application_code.base, slot.code_origin);
    assert_eq!(layout.application_data.base, slot.data_origin);
}

#[test]
fn encodes_application_permissions_and_execution_policy() {
    let Some(code) = super::descriptor::MpuRegion::new(
        0x2000_8000,
        32_768,
        super::descriptor::MpuAccess::ReadOnly,
        super::descriptor::MpuExecution::Allowed,
    ) else {
        return;
    };
    let Some(data) = super::descriptor::MpuRegion::new(
        0x2001_0000,
        32_768,
        super::descriptor::MpuAccess::ReadWrite,
        super::descriptor::MpuExecution::Never,
    ) else {
        return;
    };

    assert_eq!(
        code.rasr_bits() & super::descriptor::AP_READ_ONLY,
        super::descriptor::AP_READ_ONLY
    );
    assert_eq!(code.rasr_bits() & super::descriptor::RASR_XN, 0);
    assert_eq!(
        code.rasr_bits() & super::descriptor::RASR_ENABLE,
        super::descriptor::RASR_ENABLE
    );
    assert_eq!(
        data.rasr_bits() & super::descriptor::AP_READ_WRITE,
        super::descriptor::AP_READ_WRITE
    );
    assert_eq!(
        data.rasr_bits() & super::descriptor::RASR_XN,
        super::descriptor::RASR_XN
    );
}
