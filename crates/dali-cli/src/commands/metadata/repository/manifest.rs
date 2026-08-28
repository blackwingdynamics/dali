use std::fs;

use dali_metadata::BoundedText;

use super::common;

pub(super) struct CartridgeManifest {
    pub(super) cartridge_id: [u8; 16],
    pub(super) developer_key_id: dali_metadata::KeyId,
    pub(super) target_profile: BoundedText<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>,
    pub(super) cartridge_version: BoundedText<{ dali_metadata::MAX_CARTRIDGE_VERSION_BYTES }>,
    pub(super) minimum_kernel_version: BoundedText<{ dali_metadata::MAX_CARTRIDGE_VERSION_BYTES }>,
    pub(super) cartridge_version_parts: dali_amrn::v4::Version,
    pub(super) minimum_kernel_version_parts: dali_amrn::v4::Version,
    pub(super) required_services: u32,
    pub(super) slot_id: u8,
}

pub(super) fn parse(path: &std::path::Path) -> Result<CartridgeManifest, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("cannot read manifest {}: {error}", path.display()))?;
    let target_profile = common::bounded::<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>(
        &value(&contents, "target_profile")?,
        "target_profile",
    )?;
    let cartridge_version_value = value(&contents, "version")?;
    let cartridge_version_parts = version(&cartridge_version_value, "version")?;
    let cartridge_version = common::bounded::<{ dali_metadata::MAX_CARTRIDGE_VERSION_BYTES }>(
        &cartridge_version_value,
        "version",
    )?;
    let minimum_kernel_version_value = value(&contents, "minimum_kernel_version")?;
    let minimum_kernel_version_parts =
        version(&minimum_kernel_version_value, "minimum_kernel_version")?;
    let minimum_kernel_version = common::bounded::<{ dali_metadata::MAX_CARTRIDGE_VERSION_BYTES }>(
        &minimum_kernel_version_value,
        "minimum_kernel_version",
    )?;
    let cartridge_id = common::parse_hex::<16>(&value(&contents, "cartridge_id")?, "cartridge_id")?;
    let developer_key_id = dali_metadata::KeyId(common::parse_hex::<16>(
        &value(&contents, "signing_key_id")?,
        "signing_key_id",
    )?);
    let required_services = integer(&value(&contents, "required_services")?)?;
    let abi_version = integer(&value(&contents, "abi_version")?)?;
    if abi_version != u32::from(dali_amrn::v3::ABI_VERSION) {
        return Err(format!(
            "only ABI {} cartridges are supported",
            dali_amrn::v3::ABI_VERSION
        ));
    }
    let slot_name = value(&contents, "slot")?;
    let target = dali_targets::find_target(target_profile.as_str().unwrap_or_default())
        .ok_or_else(|| "manifest target_profile is not a supported target".to_owned())?;
    let slot = target
        .memory
        .isolation
        .and_then(|memory| memory.slots.iter().find(|slot| slot.name == slot_name))
        .ok_or_else(|| format!("target does not declare application slot `{slot_name}`"))?;
    Ok(CartridgeManifest {
        cartridge_id,
        developer_key_id,
        target_profile,
        cartridge_version,
        minimum_kernel_version,
        cartridge_version_parts,
        minimum_kernel_version_parts,
        required_services,
        slot_id: slot.id,
    })
}

fn value(contents: &str, key: &str) -> Result<String, String> {
    contents
        .lines()
        .map(str::trim)
        .filter_map(|line| line.split_once('='))
        .find_map(|(name, value)| {
            (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
        })
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("manifest is missing `{key}`"))
}

fn integer(value: &str) -> Result<u32, String> {
    let value = value.trim();
    let parsed = value
        .strip_prefix("0x")
        .map_or_else(|| value.parse(), |value| u32::from_str_radix(value, 16));
    parsed.map_err(|_| format!("manifest contains invalid integer `{value}`"))
}

fn version(value: &str, field: &str) -> Result<dali_amrn::v4::Version, String> {
    let mut parts = value.split('.');
    let values = [parts.next(), parts.next(), parts.next()];
    if parts.next().is_some() || values.iter().any(Option::is_none) {
        return Err(format!("manifest `{field}` must use major.minor.patch"));
    }
    let parse = |value: Option<&str>| {
        value
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| format!("manifest `{field}` contains an invalid version"))
    };
    Ok(dali_amrn::v4::Version {
        major: parse(values[0])?,
        minor: parse(values[1])?,
        patch: parse(values[2])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identity_and_slot_fields() {
        let path = std::env::temp_dir().join(format!(
            "dali-register-cartridge-manifest-{}",
            std::process::id()
        ));
        fs::write(
            &path,
            "target_profile = \"f405\"\nversion = \"0.1.0\"\nminimum_kernel_version = \"0.1.0\"\ncartridge_id = \"00112233445566778899AABBCCDDEEFF\"\nsigning_key_id = \"0125BBC1A5334433E03D43C9AAD40DF3\"\nrequired_services = \"0x1\"\nabi_version = 3\nslot = \"slot1\"\n",
        )
        .expect("write manifest");
        let manifest = parse(&path).expect("parse manifest");
        assert_eq!(manifest.slot_id, 1);
        assert_eq!(manifest.required_services, 1);
        fs::remove_file(path).expect("remove manifest");
    }
}
