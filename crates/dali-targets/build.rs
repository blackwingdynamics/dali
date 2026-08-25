use std::{env, fs, path::PathBuf};

#[path = "build/generator/mod.rs"]
mod generator;
#[path = "build/loader.rs"]
mod loader;
#[path = "build/manifest.rs"]
mod manifest;
#[path = "build/render.rs"]
mod render;
#[path = "build/validation/mod.rs"]
mod validation;

use manifest::Manifest;

const MANIFEST_DIRECTORY: &str = "targets";
const GENERATED_FILE: &str = "target_profiles.rs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let manifest_directory = crate_directory.join("../..").join(MANIFEST_DIRECTORY);
    let mut paths = loader::manifest_paths(&manifest_directory)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "no target manifests found in {}",
            manifest_directory.display()
        )
        .into());
    }

    let manifests = paths
        .iter()
        .map(|path| loader::read_manifest(path))
        .collect::<Result<Vec<Manifest>, _>>()?;
    validation::validate_manifests(&manifests)?;

    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed={}", manifest_directory.display());
    let generated = generator::generate_registry(&manifests);
    let output = PathBuf::from(env::var("OUT_DIR")?).join(GENERATED_FILE);
    fs::write(output, generated)?;
    Ok(())
}
