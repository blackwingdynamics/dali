use std::{fs, path::PathBuf};

use super::manifest::Manifest;

pub(super) fn manifest_paths(
    directory: &PathBuf,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let entries = fs::read_dir(directory)?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub(super) fn read_manifest(path: &PathBuf) -> Result<Manifest, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let manifest = toml::from_str(&contents)
        .map_err(|error| format!("invalid target manifest {}: {error}", path.display()))?;
    Ok(manifest)
}
