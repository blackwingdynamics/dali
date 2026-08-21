use std::{env, fs, path::PathBuf};

use dali_targets::SUPPORTED_TARGETS;

const FAULT_CAPTURE_REGION_LENGTH: u32 = 128;

fn main() {
    let target = SUPPORTED_TARGETS
        .first()
        .expect("an application-supported target is required");
    let dma = target.memory.dma;
    let runtime = dali_targets::TargetMemoryRegion {
        origin: target.memory.runtime_origin,
        length: target.memory.runtime_length,
    };
    let ccm = target
        .memory
        .ccm
        .expect("the active target must declare CCM runtime memory");
    assert!(
        ccm.length > FAULT_CAPTURE_REGION_LENGTH,
        "the active target CCM must reserve space for retained fault evidence"
    );
    let retained = dali_targets::TargetMemoryRegion {
        origin: ccm.origin,
        length: FAULT_CAPTURE_REGION_LENGTH,
    };
    let ram = dali_targets::TargetMemoryRegion {
        origin: ccm.origin + FAULT_CAPTURE_REGION_LENGTH,
        length: ccm.length - FAULT_CAPTURE_REGION_LENGTH,
    };
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is required"));
    let linker_script = render_linker_script(target.memory.flash, ram, dma, runtime, retained);
    fs::write(output_directory.join("memory.x"), linker_script)
        .expect("generated memory.x must be writable");
    println!("cargo:rustc-link-search={}", output_directory.display());
    println!("cargo:rerun-if-changed=../targets");
}

fn render_linker_script(
    flash: dali_targets::TargetMemoryRegion,
    ram: dali_targets::TargetMemoryRegion,
    dma: dali_targets::TargetMemoryRegion,
    runtime: dali_targets::TargetMemoryRegion,
    retained: dali_targets::TargetMemoryRegion,
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
