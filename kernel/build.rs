use std::{env, fs, path::PathBuf};

use dali_targets::SUPPORTED_TARGETS;

fn main() {
    let target = SUPPORTED_TARGETS
        .first()
        .expect("an application-supported target is required");
    let dma = target.memory.dma;
    let ccm = target
        .memory
        .ccm
        .expect("the active target must declare CCM runtime memory");
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is required"));
    let linker_script = render_linker_script(target.memory.flash, dma, ccm);
    fs::write(output_directory.join("memory.x"), linker_script)
        .expect("generated memory.x must be writable");
    println!("cargo:rustc-link-search={}", output_directory.display());
    println!("cargo:rerun-if-changed=../targets");
}

fn render_linker_script(
    flash: dali_targets::TargetMemoryRegion,
    dma: dali_targets::TargetMemoryRegion,
    ccm: dali_targets::TargetMemoryRegion,
) -> String {
    format!(
        "MEMORY\n{{\n    FLASH (rx) : ORIGIN = 0x{:08X}, LENGTH = {}\n    RAM (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n    DMA (xrw) : ORIGIN = 0x{:08X}, LENGTH = {}\n}}\n\nSECTIONS\n{{\n    .dma_buffer (NOLOAD) : ALIGN(4)\n    {{\n        KEEP(*(.dma_buffer))\n    }} > DMA\n}}\n",
        flash.origin, flash.length, ccm.origin, ccm.length, dma.origin, dma.length,
    )
}
