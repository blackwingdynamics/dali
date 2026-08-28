mod authentication;
mod hardware;
mod profile;

use self::profile::generate_profile;
use super::manifest::Manifest;
use crate::render::constant_name;

pub(super) fn generate_registry(manifests: &[Manifest]) -> String {
    let definitions = manifests
        .iter()
        .map(generate_profile)
        .collect::<Vec<_>>()
        .join("\n");
    let capacities = manifests
        .iter()
        .map(|manifest| {
            let constant = constant_name(&manifest.profile.name);
            let capacity = manifest
                .memory
                .isolation
                .as_ref()
                .map_or(0, |isolation| isolation.slots.len());
            format!("pub const {constant}_CONTEXT_CAPACITY: usize = {capacity};")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let names = manifests
        .iter()
        .map(|manifest| constant_name(&manifest.profile.name))
        .collect::<Vec<_>>()
        .join(", ");
    let supported = manifests
        .iter()
        .filter(|manifest| manifest.profile.application_supported)
        .map(|manifest| constant_name(&manifest.profile.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{definitions}\n\n{capacities}\n\npub const ALL_TARGETS: &[TargetProfile] = &[{names}];\npub const SUPPORTED_TARGETS: &[TargetProfile] = &[{supported}];\n"
    )
}
