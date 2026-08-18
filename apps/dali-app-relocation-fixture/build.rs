use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let linker_script = output_directory.join("link.x");
    let target = dali_targets::SUPPORTED_TARGETS
        .first()
        .ok_or("no application-supported target profile is declared")?;
    let isolation = target
        .memory
        .isolation
        .ok_or("target isolation metadata is missing")?;
    let slot = isolation.active_slot().ok_or("active isolation slot is missing")?;
    let linker_script_contents = format!(
        "MEMORY\n{{\n    DALI_CODE (rx) : ORIGIN = 0x{:08X}, LENGTH = {}\n    DALI_DATA (rw) : ORIGIN = 0x{:08X}, LENGTH = {}\n}}\n\nENTRY(amiran_entry)\n\nSECTIONS\n{{\n    .dali_code : ALIGN(4)\n    {{\n        KEEP(*(.text.amiran_entry))\n        *(.text*)\n        *(.rodata*)\n    }} > DALI_CODE\n\n    .dali_data : ALIGN(4)\n    {{\n        __dali_data_start = .;\n        *(.data*)\n        __dali_data_end = .;\n    }} > DALI_DATA\n\n    .dali_bss (NOLOAD) : ALIGN(4)\n    {{\n        __dali_bss_start = .;\n        *(.bss*)\n        *(COMMON)\n        __dali_bss_end = .;\n    }} > DALI_DATA\n\n    /DISCARD/ :\n    {{\n        *(.ARM.exidx*)\n        *(.ARM.extab*)\n        *(.eh_frame*)\n    }}\n}}\n",
        slot.code_origin, slot.code_length, slot.data_origin, slot.data_length,
    );
    fs::write(&linker_script, linker_script_contents)?;
    println!("cargo:rustc-link-search={}", output_directory.display());
    println!("cargo:rustc-link-arg=--emit-relocs");
    println!("cargo:rerun-if-changed=../../targets");

    Ok(())
}
