use super::super::manifest::{IsolationMemory, Manifest};

const MINIMUM_MPU_REGION_BYTES: u32 = 32;

pub(super) fn validate_memory_regions(
    manifest: &Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let regions = [
        ("flash", &manifest.memory.flash),
        ("dma", &manifest.memory.dma),
    ];
    for (name, region) in regions {
        if region.length == 0 || region.origin.checked_add(region.length).is_none() {
            return Err(format!(
                "target {} has invalid {name} memory bounds",
                manifest.profile.name
            )
            .into());
        }
    }
    if let Some(region) = &manifest.memory.ccm
        && (region.length == 0 || region.origin.checked_add(region.length).is_none())
    {
        return Err(format!(
            "target {} has invalid ccm memory bounds",
            manifest.profile.name
        )
        .into());
    }
    let kernel_end = validate_named_region(
        manifest,
        "kernel",
        manifest.memory.kernel_origin,
        manifest.memory.kernel_length,
    )?;
    let application_end = validate_named_region(
        manifest,
        "application",
        manifest.memory.application_origin,
        manifest.memory.application_length,
    )?;
    validate_named_region(
        manifest,
        "runtime",
        manifest.memory.runtime_origin,
        manifest.memory.runtime_length,
    )?;
    if kernel_end != manifest.memory.application_origin
        || application_end != manifest.memory.runtime_origin
    {
        return Err(format!(
            "target {} kernel, application, and runtime regions must be contiguous",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn validate_named_region(
    manifest: &Manifest,
    name: &str,
    origin: u32,
    length: u32,
) -> Result<u32, Box<dyn std::error::Error>> {
    if length == 0 {
        return Err(format!("target {} has empty {name} memory", manifest.profile.name).into());
    }
    origin.checked_add(length).ok_or_else(|| {
        format!(
            "target {} has overflowing {name} memory",
            manifest.profile.name
        )
        .into()
    })
}

pub(super) fn validate_isolation_memory(
    manifest: &Manifest,
    isolation: &IsolationMemory,
) -> Result<(), Box<dyn std::error::Error>> {
    let application_end = manifest
        .memory
        .application_origin
        .checked_add(manifest.memory.application_length)
        .ok_or_else(|| {
            format!(
                "target {} application memory overflows",
                manifest.profile.name
            )
        })?;
    match (isolation.peripheral_origin, isolation.peripheral_length) {
        (Some(origin), Some(length)) if valid_mpu_region(origin, length) => {}
        _ => {
            return Err(format!(
                "target {} isolation memory must declare an aligned peripheral region",
                manifest.profile.name
            )
            .into());
        }
    }
    match (isolation.bus_fault_origin, isolation.bus_fault_length) {
        (Some(origin), Some(length)) if valid_mpu_region(origin, length) => {}
        (None, None) => {}
        _ => {
            return Err(format!(
                "target {} BusFault fixture range must be declared as an aligned pair",
                manifest.profile.name
            )
            .into());
        }
    }
    validate_slots(manifest, isolation, application_end)
}

fn validate_slots(
    manifest: &Manifest,
    isolation: &IsolationMemory,
    application_end: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    for (index, slot) in isolation.slots.iter().enumerate() {
        let code_end = slot
            .code_origin
            .checked_add(slot.code_length)
            .ok_or_else(|| {
                format!(
                    "target {} slot {index} code overflows",
                    manifest.profile.name
                )
            })?;
        let data_end = slot
            .data_origin
            .checked_add(slot.data_length)
            .ok_or_else(|| {
                format!(
                    "target {} slot {index} data overflows",
                    manifest.profile.name
                )
            })?;
        if slot.name.is_empty()
            || slot.code_length == 0
            || slot.data_length == 0
            || slot.stack_length == 0
            || slot.stack_length > slot.data_length
            || slot.data_origin != code_end
            || slot.code_origin < manifest.memory.application_origin
            || data_end > application_end
            || !valid_mpu_region(slot.code_origin, slot.code_length)
            || !valid_mpu_region(slot.data_origin, slot.data_length)
        {
            return Err(format!(
                "target {} slot {index} has invalid aligned code/data bounds",
                manifest.profile.name
            )
            .into());
        }
        for previous in &isolation.slots[..index] {
            let previous_end = previous
                .data_origin
                .checked_add(previous.data_length)
                .ok_or_else(|| format!("target {} slot bounds overflow", manifest.profile.name))?;
            if slot.id == previous.id
                || slot.name == previous.name
                || slot.code_origin != previous_end
            {
                return Err(format!(
                    "target {} has duplicate or overlapping isolation slots",
                    manifest.profile.name
                )
                .into());
            }
        }
    }
    let Some(first) = isolation.slots.first() else {
        return Err(format!(
            "target {} isolation memory must declare at least one slot",
            manifest.profile.name
        )
        .into());
    };
    let last_end = isolation
        .slots
        .last()
        .and_then(|slot| slot.data_origin.checked_add(slot.data_length));
    if first.code_origin != manifest.memory.application_origin || last_end != Some(application_end)
    {
        return Err(format!(
            "target {} isolation slots must cover application memory contiguously",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn valid_mpu_region(origin: u32, length: u32) -> bool {
    length >= MINIMUM_MPU_REGION_BYTES
        && (length & (length - 1)) == 0
        && origin.is_multiple_of(length)
}
