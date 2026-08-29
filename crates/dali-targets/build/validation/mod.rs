mod memory;
mod security;

use super::manifest::Manifest;

pub(super) fn validate_manifests(manifests: &[Manifest]) -> Result<(), Box<dyn std::error::Error>> {
    for (index, manifest) in manifests.iter().enumerate() {
        memory::validate_memory_regions(manifest)?;
        validate_peripheral_metadata(manifest)?;
        validate_profile(manifest, index)?;
        validate_optional_sections(manifest)?;
        if let Some(isolation) = &manifest.memory.isolation {
            memory::validate_isolation_memory(manifest, isolation)?;
        }
        validate_unique_profile(manifests, manifest, index)?;
    }
    Ok(())
}

fn validate_peripheral_metadata(manifest: &Manifest) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.i2c.bus_frequency_hz == 0 {
        return Err(format!(
            "target manifest {} has an invalid I2C bus frequency",
            manifest.profile.name
        )
        .into());
    }
    if manifest.display.controller.is_empty()
        || manifest.display.i2c_address > (u8::MAX >> 1)
        || manifest.display.width == 0
        || manifest.display.height == 0
        || manifest.display.text_cell_width == 0
        || manifest.display.text_cell_height == 0
        || !manifest
            .display
            .width
            .is_multiple_of(manifest.display.text_cell_width)
        || !manifest
            .display
            .height
            .is_multiple_of(manifest.display.text_cell_height)
        || manifest.display.timeout_ticks == 0
    {
        return Err(format!(
            "target manifest {} has an invalid display configuration",
            manifest.profile.name
        )
        .into());
    }
    if manifest.driver_probe.uart_baud_rate_hz == 0
        || manifest.driver_probe.spi_clock_hz == 0
        || manifest.driver_probe.timeout_ticks == 0
        || manifest.driver_probe.polls_per_timeout_tick == 0
        || manifest.driver_probe.timer_timeout_divisor == 0
        || manifest.driver_probe.timer_evidence_poll_limit == 0
        || manifest.driver_probe.buffer_length == 0
        || manifest.driver_probe.i2c_address > (u8::MAX >> 1)
    {
        return Err(format!(
            "target manifest {} has an invalid driver probe configuration",
            manifest.profile.name
        )
        .into());
    }
    if manifest.user_key.port.chars().count() != 1 || manifest.user_key.pin > 15 {
        return Err(format!(
            "target manifest {} has an invalid user-key mapping",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn validate_profile(manifest: &Manifest, index: usize) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.profile.name.is_empty()
        || manifest.profile.backend.is_empty()
        || manifest.profile.board.is_empty()
        || manifest.profile.mcu.is_empty()
        || manifest.profile.rust_target.is_empty()
    {
        return Err(format!("target manifest {index} has an empty profile field").into());
    }
    if manifest.profile.application_supported
        && (manifest.profile.abi_version == 0 || manifest.profile.amrn_target_id == 0)
    {
        return Err(format!(
            "target manifest {} has an invalid contract identifier",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn validate_optional_sections(manifest: &Manifest) -> Result<(), Box<dyn std::error::Error>> {
    security::validate_authentication(&manifest.authentication, &manifest.profile.name)?;
    if let Some(artifacts) = &manifest.artifacts
        && (artifacts.kernel_binary.is_empty() || artifacts.kernel_elf.is_empty())
    {
        return Err(format!(
            "target manifest {} has an empty kernel artifact",
            manifest.profile.name
        )
        .into());
    }
    if manifest.profile.application_supported
        && manifest
            .scheduler
            .as_ref()
            .is_none_or(|scheduler| scheduler.quantum_ticks == 0)
    {
        return Err(format!(
            "target manifest {} has no valid scheduler quantum",
            manifest.profile.name
        )
        .into());
    }
    if manifest.profile.application_supported
        && manifest
            .scheduler
            .as_ref()
            .is_none_or(|scheduler| scheduler.tick_hz == 0)
    {
        return Err(format!(
            "target manifest {} has no valid scheduler tick frequency",
            manifest.profile.name
        )
        .into());
    }
    if let Some(dfu) = &manifest.profile.dfu
        && (dfu.vendor_id == 0 || dfu.product_id == 0 || dfu.address == 0)
    {
        return Err(format!(
            "target manifest {} has an invalid DFU configuration",
            manifest.profile.name
        )
        .into());
    }
    if let Some(storage) = &manifest.storage
        && (storage.bus_width == 0
            || storage.bus_width > 4
            || storage.data_timeout_cycles == 0
            || storage.command_poll_limit == 0
            || storage.data_poll_limit == 0
            || storage.dma_stop_poll_limit == 0)
    {
        return Err(format!(
            "target manifest {} has an invalid storage width",
            manifest.profile.name
        )
        .into());
    }
    if manifest.capabilities.storage != manifest.storage.is_some() {
        return Err(format!(
            "target manifest {} must align storage capability with the storage section",
            manifest.profile.name
        )
        .into());
    }
    if manifest.capabilities.mpu && manifest.memory.isolation.is_none() {
        return Err(format!(
            "target manifest {} declares MPU without isolation memory",
            manifest.profile.name
        )
        .into());
    }
    if manifest.capabilities.relocation && manifest.memory.isolation.is_none() {
        return Err(format!(
            "target manifest {} declares relocation without isolation memory",
            manifest.profile.name
        )
        .into());
    }
    Ok(())
}

fn validate_unique_profile(
    manifests: &[Manifest],
    manifest: &Manifest,
    index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for other in &manifests[..index] {
        if other.profile.name == manifest.profile.name {
            return Err(format!("duplicate target profile `{}`", manifest.profile.name).into());
        }
        if other.profile.backend == manifest.profile.backend {
            return Err(format!("duplicate backend id `{}`", manifest.profile.backend).into());
        }
        if manifest.profile.amrn_target_id != 0
            && other.profile.amrn_target_id == manifest.profile.amrn_target_id
        {
            return Err(format!("duplicate AMRN target id for `{}`", manifest.profile.name).into());
        }
    }
    Ok(())
}
