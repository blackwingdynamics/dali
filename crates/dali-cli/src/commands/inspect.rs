use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::{INPUT_FLAG, required_flag};

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let input = match arguments.len() {
        1 => discover_default_input(
            &env::current_dir()
                .map_err(|error| format!("cannot determine current directory: {error}"))?,
        )?,
        3 if arguments.get(1).map(String::as_str) == Some(INPUT_FLAG) => {
            PathBuf::from(required_flag(arguments, INPUT_FLAG)?)
        }
        _ => return Err(super::usage()),
    };
    println!("{}", inspect_package(&input)?);
    Ok(())
}

fn discover_default_input(directory: &Path) -> Result<PathBuf, String> {
    let manifest = directory.join(MANIFEST_FILE);
    if manifest.is_file() {
        return manifest_package_path(directory);
    }
    find_single_package(directory)
}

fn manifest_package_path(directory: &Path) -> Result<PathBuf, String> {
    let contents = fs::read_to_string(directory.join(MANIFEST_FILE))
        .map_err(|error| format!("cannot read {MANIFEST_FILE}: {error}"))?;
    let name = manifest_value(&contents, "name")?;
    let target_profile = manifest_value(&contents, "target_profile")?;
    let target = dali_targets::find_target(&target_profile)
        .ok_or_else(|| format!("unsupported target profile `{target_profile}`"))?
        .rust_target;
    let profile =
        manifest_value(&contents, "profile").unwrap_or_else(|_| DEVELOPMENT_PROFILE.to_owned());
    let output_directory = match profile.as_str() {
        DEVELOPMENT_PROFILE => DEBUG_OUTPUT_DIRECTORY,
        RELEASE_PROFILE => RELEASE_PROFILE,
        _ => return Err(format!("unsupported Cargo profile `{profile}`")),
    };
    Ok(directory
        .join(TARGET_DIRECTORY)
        .join(target)
        .join(output_directory)
        .join(format!("{name}.{AMRN_EXTENSION}")))
}

fn find_single_package(directory: &Path) -> Result<PathBuf, String> {
    let mut packages = fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some(AMRN_EXTENSION)
        })
        .collect::<Vec<_>>();
    match packages.len() {
        1 => Ok(packages.remove(0)),
        0 => Err(format!(
            "no `{AMRN_EXTENSION}` package found; pass {INPUT_FLAG} <package>"
        )),
        _ => Err(format!(
            "multiple `{AMRN_EXTENSION}` packages found; pass {INPUT_FLAG} <package>"
        )),
    }
}

fn manifest_value(contents: &str, key: &str) -> Result<String, String> {
    contents
        .lines()
        .find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            (candidate.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
        })
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{MANIFEST_FILE} is missing `{key}`"))
}

fn inspect_package(input: &Path) -> Result<String, String> {
    let package = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    inspect_bytes(&package)
}

const MANIFEST_FILE: &str = "dali.toml";
const TARGET_DIRECTORY: &str = "target";
const DEVELOPMENT_PROFILE: &str = "dev";
const DEBUG_OUTPUT_DIRECTORY: &str = "debug";
const RELEASE_PROFILE: &str = "release";
const AMRN_EXTENSION: &str = "amrn";

fn inspect_bytes(package: &[u8]) -> Result<String, String> {
    if package.get(4) == Some(&dali_amrn::v4::FORMAT_VERSION) {
        return inspect_identity_package(package);
    }
    if package.get(4) == Some(&dali_amrn::v3::FORMAT_VERSION) {
        return inspect_relocatable_package(package);
    }
    if package.get(4) == Some(&dali_amrn::v2::FORMAT_VERSION) {
        return inspect_isolation_package(package);
    }
    let header = dali_amrn::parse_header(package)
        .map_err(|error| format!("invalid AMRN package: {error:?}"))?;
    let expected_size = dali_amrn::HEADER_SIZE
        .checked_add(header.payload_size as usize)
        .ok_or_else(|| "package size overflow".to_owned())?;
    if package.len() != expected_size {
        return Err(format!(
            "invalid AMRN package: file length is {}, expected {expected_size}",
            package.len()
        ));
    }
    let parsed =
        dali_amrn::parse(package).map_err(|error| format!("invalid AMRN package: {error:?}"))?;
    Ok(format!(
        "AMRN package valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\npayload_size: {}\nload_address: 0x{:08X}\nexecution_offset: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
        header.format_version,
        header.target_id,
        header.header_size,
        header.payload_size,
        header.load_address,
        header.execution_offset,
        parsed.entry_address,
        header.abi_version,
        header.crc32
    ))
}

fn inspect_identity_package(package: &[u8]) -> Result<String, String> {
    let target_id = *package
        .get(5)
        .ok_or_else(|| "invalid AMRN package: truncated target identifier".to_owned())?;
    let target = dali_targets::find_by_amrn_target_id(target_id)
        .ok_or_else(|| format!("unsupported AMRN target identifier 0x{target_id:02X}"))?;
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| format!("target {} has no ABI v3 memory contract", target.name))?;
    let mut parsed = None;
    let mut last_error = dali_amrn::v4::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = dali_amrn::v3::Contract {
            target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match dali_amrn::v4::parse(package, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN package: {last_error:?}"))?;
    let metadata = parsed.header.metadata;
    Ok(format!(
        "AMRN package valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\npackage_id: {:02X?}\npackage_version: {}.{}.{}\nminimum_kernel_version: {}.{}.{}\nrequired_services: 0x{:08X}\nslot_id: {}\ncode_size: {}\ndata_init_size: {}\nrelocation_count: {}\nabi_version: {}\npackage_crc32: 0x{:08X}",
        dali_amrn::v4::FORMAT_VERSION,
        parsed.header.image.target_id,
        dali_amrn::v4::HEADER_SIZE,
        metadata.package_id,
        metadata.package_version.major,
        metadata.package_version.minor,
        metadata.package_version.patch,
        metadata.minimum_kernel_version.major,
        metadata.minimum_kernel_version.minor,
        metadata.minimum_kernel_version.patch,
        metadata.required_services,
        metadata.slot_id,
        parsed.header.image.code_size,
        parsed.header.image.data_init_size,
        parsed.header.image.relocation_count,
        dali_amrn::v4::ABI_VERSION,
        parsed.header.package_crc32,
    ))
}

fn inspect_relocatable_package(package: &[u8]) -> Result<String, String> {
    let target_id = *package
        .get(5)
        .ok_or_else(|| "invalid AMRN package: truncated target identifier".to_owned())?;
    let target = dali_targets::find_by_amrn_target_id(target_id)
        .ok_or_else(|| format!("unsupported AMRN target identifier 0x{target_id:02X}"))?;
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| format!("target {} has no ABI v3 memory contract", target.name))?;
    let mut parsed = None;
    let mut last_error = dali_amrn::v3::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = dali_amrn::v3::Contract {
            target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match dali_amrn::v3::parse(package, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN package: {last_error:?}"))?;
    Ok(format!(
        "AMRN package valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncode_size: {}\ndata_init_size: {}\ndata_zero_size: {}\nstack_size: {}\nlinked_code_base: 0x{:08X}\nlinked_data_base: 0x{:08X}\ncode_load_address: 0x{:08X}\ndata_load_address: 0x{:08X}\nexecution_offset: {}\nrelocation_count: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
        dali_amrn::v3::FORMAT_VERSION,
        parsed.header.target_id,
        dali_amrn::v3::HEADER_SIZE,
        parsed.header.code_size,
        parsed.header.data_init_size,
        parsed.header.data_zero_size,
        parsed.header.stack_size,
        parsed.header.linked_code_base,
        parsed.header.linked_data_base,
        parsed.header.code_load_address,
        parsed.header.data_load_address,
        parsed.header.execution_offset,
        parsed.header.relocation_count,
        parsed
            .header
            .code_load_address
            .checked_add(parsed.header.execution_offset)
            .ok_or_else(|| "AMRN v3 entry address overflow".to_owned())?,
        dali_amrn::v3::ABI_VERSION,
        parsed.header.crc32,
    ))
}

fn inspect_isolation_package(package: &[u8]) -> Result<String, String> {
    let target_id = *package
        .get(5)
        .ok_or_else(|| "invalid AMRN package: truncated target identifier".to_owned())?;
    let target = dali_targets::find_by_amrn_target_id(target_id)
        .ok_or_else(|| format!("unsupported AMRN target identifier 0x{target_id:02X}"))?;
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| format!("target `{}` has no ABI v3 memory contract", target.name))?;
    let mut parsed = None;
    let mut last_error = dali_amrn::v2::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = dali_amrn::v2::Contract {
            target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match dali_amrn::v2::parse(package, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN package: {last_error:?}"))?;
    Ok(format!(
        "AMRN package valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncode_size: {}\ndata_init_size: {}\ndata_zero_size: {}\nstack_size: {}\ncode_load_address: 0x{:08X}\ndata_load_address: 0x{:08X}\nexecution_offset: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
        dali_amrn::v2::FORMAT_VERSION,
        parsed.header.target_id,
        dali_amrn::v2::HEADER_SIZE,
        parsed.header.code_size,
        parsed.header.data_init_size,
        parsed.header.data_zero_size,
        parsed.header.stack_size,
        parsed.header.code_load_address,
        parsed.header.data_load_address,
        parsed.header.execution_offset,
        parsed.entry_address,
        dali_amrn::v2::ABI_VERSION,
        parsed.header.crc32,
    ))
}

#[cfg(test)]
mod tests {
    use super::inspect_bytes;

    #[test]
    fn inspect_reports_contract_fields_for_valid_package() {
        let package = test_package();
        let result = inspect_bytes(&package);
        assert!(result.is_ok(), "package should inspect");
        let report = result.unwrap_or_default();

        assert!(report.contains("AMRN package valid"));
        assert!(report.contains("target_id: 0x02"));
        assert!(report.contains("abi_version: 2"));
        assert!(report.contains("payload_size: 4"));
    }

    #[test]
    fn inspect_rejects_trailing_bytes() {
        let mut package = test_package();
        package.push(0);

        let result = inspect_bytes(&package);
        assert!(result.is_err(), "trailing data must fail");
        let error = match result {
            Ok(_) => String::new(),
            Err(error) => error,
        };

        assert!(error.contains("file length"));
    }

    fn test_package() -> Vec<u8> {
        let payload = [0x00, 0xBF, 0x00, 0xBF];
        let mut package = vec![0; dali_amrn::HEADER_SIZE + payload.len()];
        let result = dali_amrn::encode_package(&payload, 0, &mut package);
        assert!(result.is_ok(), "test payload must produce a valid package");
        let written = result.unwrap_or_default();
        package.truncate(written);
        package
    }

    #[test]
    fn inspect_reports_isolation_contract_fields() {
        let contract = v2_test_contract();
        let image = dali_amrn::v2::Image {
            code: &[0, 191, 0, 191],
            initialized_data: &[1, 2, 3, 4],
            data_zero_size: 8,
            stack_size: 16,
            execution_offset: 0,
        };
        let mut package = vec![0; dali_amrn::v2::HEADER_SIZE + 8];
        let written = dali_amrn::v2::encode(image, contract, &mut package)
            .expect("v2 test package should encode");
        package.truncate(written);
        let report = inspect_bytes(&package).expect("v2 package should inspect");
        assert!(report.contains("format_version: 2"));
        assert!(report.contains("abi_version: 3"));
        assert!(report.contains("data_zero_size: 8"));
    }

    fn v2_test_contract() -> dali_amrn::v2::Contract {
        dali_amrn::v2::Contract {
            target_id: 2,
            code_load_address: 0x2000_8000,
            code_capacity: 16 * 1024,
            data_load_address: 0x2000_C000,
            data_capacity: 16 * 1024,
        }
    }

    #[test]
    fn inspect_reports_identity_fields() {
        let contract = dali_amrn::v3::Contract {
            target_id: 2,
            code_load_address: 0x2001_0000,
            code_capacity: 16 * 1024,
            data_load_address: 0x2001_4000,
            data_capacity: 16 * 1024,
        };
        let image = dali_amrn::v4::Image {
            image: dali_amrn::v3::Image {
                code: &[0, 0, 0, 0],
                initialized_data: &[],
                data_zero_size: 0,
                stack_size: 4096,
                linked_code_base: 0x2000_8000,
                linked_data_base: 0x2000_C000,
                execution_offset: 0,
                relocations: &[],
            },
            metadata: dali_amrn::v4::Metadata {
                package_id: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                package_version: dali_amrn::v4::Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
                minimum_kernel_version: dali_amrn::v4::Version {
                    major: 0,
                    minor: 1,
                    patch: 0,
                },
                required_services: 1,
                slot_id: 1,
            },
        };
        let mut package = vec![0; dali_amrn::v4::HEADER_SIZE + 4];
        let size =
            dali_amrn::v4::encode(image, contract, &mut package).expect("v4 package should encode");
        package.truncate(size);
        let report = inspect_bytes(&package).expect("v4 package should inspect");
        assert!(report.contains("format_version: 4"));
        assert!(report.contains("package_version: 1.2.3"));
        assert!(report.contains("slot_id: 1"));
    }
}

#[cfg(test)]
#[path = "inspect_v3_tests.rs"]
mod v3_tests;
