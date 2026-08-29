use std::{env, fs, io, path::PathBuf};

use dali_targets::{SUPPORTED_TARGETS, TargetMemoryRegion, TargetProfile};

const BACKEND_ID: &str = "stm32f405";
const TARGET_PROFILE_ENV: &str = "DALI_TARGET_PROFILE";
const FAULT_CAPTURE_REGION_LENGTH: u32 = 128;

fn main() -> Result<(), io::Error> {
    println!("cargo:rerun-if-env-changed={TARGET_PROFILE_ENV}");
    println!("cargo:rerun-if-changed=../../targets");

    let target = selected_target()?;
    let linker_script = render_linker_script(target)?;
    let output_directory = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR is required"))?,
    );
    fs::write(output_directory.join("memory.x"), linker_script)?;
    println!("cargo:root={}", output_directory.display());
    Ok(())
}

fn selected_target() -> Result<&'static TargetProfile, io::Error> {
    if let Ok(name) = env::var(TARGET_PROFILE_ENV) {
        let target = dali_targets::find_target(&name).ok_or_else(|| {
            io::Error::other(format!(
                "DALI_TARGET_PROFILE={name:?} does not name an application-supported target"
            ))
        })?;
        return ensure_backend(target);
    }

    let mut matching = SUPPORTED_TARGETS
        .iter()
        .filter(|target| target.backend == BACKEND_ID);
    let target = matching
        .next()
        .ok_or_else(|| io::Error::other("an application-supported F405 target is required"))?;
    if matching.next().is_some() {
        return Err(io::Error::other(format!(
            "multiple {BACKEND_ID} targets exist; set {TARGET_PROFILE_ENV} explicitly"
        )));
    }
    Ok(target)
}

fn ensure_backend(target: &'static TargetProfile) -> Result<&'static TargetProfile, io::Error> {
    if target.backend == BACKEND_ID {
        Ok(target)
    } else {
        Err(io::Error::other(format!(
            "target profile {:?} belongs to backend {:?}, expected {:?}",
            target.name, target.backend, BACKEND_ID
        )))
    }
}

fn render_linker_script(target: &TargetProfile) -> Result<String, io::Error> {
    let ccm = target
        .memory
        .ccm
        .ok_or_else(|| io::Error::other("the F405 target must declare CCM runtime memory"))?;
    if ccm.length <= FAULT_CAPTURE_REGION_LENGTH {
        return Err(io::Error::other(
            "the F405 target CCM must reserve space for retained fault evidence",
        ));
    }
    let retained = TargetMemoryRegion {
        origin: ccm.origin,
        length: FAULT_CAPTURE_REGION_LENGTH,
    };
    let ram = TargetMemoryRegion {
        origin: ccm.origin + FAULT_CAPTURE_REGION_LENGTH,
        length: ccm.length - FAULT_CAPTURE_REGION_LENGTH,
    };
    let runtime = TargetMemoryRegion {
        origin: target.memory.runtime_origin,
        length: target.memory.runtime_length,
    };
    Ok(format_linker_script(
        target.memory.flash,
        ram,
        target.memory.dma,
        runtime,
        retained,
    ))
}

fn format_linker_script(
    flash: TargetMemoryRegion,
    ram: TargetMemoryRegion,
    dma: TargetMemoryRegion,
    runtime: TargetMemoryRegion,
    retained: TargetMemoryRegion,
) -> String {
    format!(
        "MEMORY\n{{\n    FLASH (rx) : ORIGIN = 0x{:08X}, LENGTH = {}\n    RAM (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n    DMA (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n    RUNTIME (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n    RETAINED (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n}}\n\nSECTIONS\n{{\n    .dma_buffer (NOLOAD) : ALIGN(4)\n    {{\n        KEEP(*(.dma_buffer))\n    }} > DMA\n\n    .fault_capture (NOLOAD) : ALIGN(4)\n    {{\n        KEEP(*(.fault_capture))\n    }} > RETAINED\n\n    .repository_workspace (NOLOAD) : ALIGN(4)\n    {{\n        KEEP(*(.repository_workspace))\n    }} > RUNTIME\n}}\n",
        flash.origin,
        flash.length,
        ram.origin,
        ram.length,
        dma.origin,
        dma.length,
        runtime.origin,
        runtime.length,
        retained.origin,
        retained.length,
    )
}
