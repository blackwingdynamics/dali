use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let linker_script = output_directory.join("link.x");
    let memory = dali_targets::TARGET_F405.memory;
    let isolation = memory
        .isolation
        .ok_or("F405 isolation metadata is missing")?;
    let slot = isolation.active_slot().ok_or("active isolation slot is missing")?;
    let linker_script_contents = format!(
        "MEMORY\n{{\n    DALI_CODE (rx) : ORIGIN = 0x{:08X}, LENGTH = {}\n    DALI_DATA (rw) : ORIGIN = 0x{:08X}, LENGTH = {}\n}}\n\nENTRY(amiran_entry)\n\nSECTIONS\n{{\n    .dali_code :\n    {{\n        . = ALIGN(4);\n        *(.text.amiran_entry)\n        *(.text*)\n        *(.rodata*)\n        . = ALIGN(4);\n    }} > DALI_CODE\n\n    .dali_data :\n    {{\n        . = ALIGN(4);\n        *(.data*)\n        . = ALIGN(4);\n        __dali_bss_start = .;\n        *(.bss*)\n        *(COMMON)\n        . = ALIGN(4);\n        __dali_bss_end = .;\n        . = ALIGN(4);\n    }} > DALI_DATA\n}}\n",
        slot.code_origin, slot.code_length, slot.data_origin, slot.data_length,
    );
    fs::write(&linker_script, linker_script_contents)?;
    println!("cargo:rustc-link-search={}", output_directory.display());
    println!("cargo:rerun-if-changed=../../targets/f405.toml");
    Ok(())
}
