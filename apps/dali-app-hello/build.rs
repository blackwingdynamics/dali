use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let linker_script = output_dir.join("link.x");
    fs::copy("memory.x", &linker_script)?;
    println!("cargo:rustc-link-search={}", output_dir.display());
    println!("cargo:rerun-if-changed=memory.x");
    Ok(())
}
