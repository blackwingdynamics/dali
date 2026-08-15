use std::{env, fs};

use super::super::package as package_command;
use super::build;

const AMRN_EXTENSION: &str = "amrn";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 2 {
        return Err(usage());
    }
    let project_directory = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let manifest = build::read_manifest(&project_directory)?;
    let release = build::cargo_profile_is_release(&manifest.profile)?;
    let payload = build::payload_path(
        &project_directory,
        &manifest.target_profile,
        release,
        &manifest.name,
    );
    let output = payload.with_extension(AMRN_EXTENSION);
    let payload_bytes = fs::read(&payload)
        .map_err(|error| format!("cannot read native payload {}: {error}", payload.display()))?;
    let package = package_command::build_package(&payload_bytes, manifest.entry_offset)?;
    fs::write(&output, package)
        .map_err(|error| format!("cannot write AMRN package {}: {error}", output.display()))?;
    println!("Created AMRN package: {}", output.display());
    Ok(())
}

fn usage() -> String {
    "usage:\n  dali app package".to_owned()
}
