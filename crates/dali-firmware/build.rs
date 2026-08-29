use std::{env, io};

const BOARD_ROOT_ENV: &str = "DEP_DALI_BOARD_STM32F405_ROOT";

fn main() -> Result<(), io::Error> {
    println!("cargo:rerun-if-env-changed={BOARD_ROOT_ENV}");
    if let Some(root) = env::var_os(BOARD_ROOT_ENV) {
        println!("cargo:rustc-link-search={}", root.to_string_lossy());
    }
    Ok(())
}
