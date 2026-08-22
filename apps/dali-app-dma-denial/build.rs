use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output_directory = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let linker_script = output_directory.join("link.x");
    let memory = dali_targets::TARGET_F405.memory;
    let isolation = memory
        .isolation
        .ok_or("F405 isolation metadata is missing")?;
    let slot = isolation
        .active_slot()
        .ok_or("active isolation slot is missing")?;
    let contents = format!(
        "MEMORY\n{{\n    DALI_CODE (rx) : ORIGIN = 0x{:08X}, LENGTH = {}\n    DALI_DATA (rw) : ORIGIN = 0x{:08X}, LENGTH = {}\n}}\n\nENTRY(amiran_entry)\n\nSECTIONS\n{{\n    .dali_code : {{ . = ALIGN(4); *(.text.amiran_entry) *(.text*) *(.rodata*) . = ALIGN(4); }} > DALI_CODE\n    .dali_data : {{ . = ALIGN(4); *(.data*) . = ALIGN(4); __dali_bss_start = .; *(.bss*) *(COMMON) . = ALIGN(4); __dali_bss_end = .; }} > DALI_DATA\n}}\n",
        slot.code_origin, slot.code_length, slot.data_origin, slot.data_length,
    );
    fs::write(linker_script, contents)?;
    println!("cargo:rustc-link-search={}", output_directory.display());
    println!("cargo:rerun-if-changed=../../targets/f405.toml");
    Ok(())
}
