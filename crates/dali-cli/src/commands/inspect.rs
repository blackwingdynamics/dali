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
    println!("{}", inspect_cartridge(&input)?);
    Ok(())
}

fn discover_default_input(directory: &Path) -> Result<PathBuf, String> {
    let manifest = directory.join(MANIFEST_FILE);
    if manifest.is_file() {
        return manifest_cartridge_path(directory);
    }
    find_single_cartridge(directory)
}

fn manifest_cartridge_path(directory: &Path) -> Result<PathBuf, String> {
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

fn find_single_cartridge(directory: &Path) -> Result<PathBuf, String> {
    let mut cartridges = fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some(AMRN_EXTENSION)
        })
        .collect::<Vec<_>>();
    match cartridges.len() {
        1 => Ok(cartridges.remove(0)),
        0 => Err(format!(
            "no `{AMRN_EXTENSION}` cartridge found; pass {INPUT_FLAG} <cartridge>"
        )),
        _ => Err(format!(
            "multiple `{AMRN_EXTENSION}` cartridges found; pass {INPUT_FLAG} <cartridge>"
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

fn inspect_cartridge(input: &Path) -> Result<String, String> {
    let cartridge = fs::read(input).map_err(|error| format!("cannot read input: {error}"))?;
    inspect_bytes(&cartridge)
}

const MANIFEST_FILE: &str = "dali.toml";
const TARGET_DIRECTORY: &str = "target";
const DEVELOPMENT_PROFILE: &str = "dev";
const DEBUG_OUTPUT_DIRECTORY: &str = "debug";
const RELEASE_PROFILE: &str = "release";
const AMRN_EXTENSION: &str = "amrn";

fn inspect_bytes(cartridge: &[u8]) -> Result<String, String> {
    if cartridge.get(dali_amrn::v5::FORMAT_VERSION_OFFSET) == Some(&dali_amrn::v5::FORMAT_VERSION) {
        return inspect_signed_cartridge(cartridge);
    }
    if cartridge.get(4) == Some(&dali_amrn::v4::FORMAT_VERSION) {
        return inspect_identity_cartridge(cartridge);
    }
    if cartridge.get(4) == Some(&dali_amrn::v3::FORMAT_VERSION) {
        return inspect_relocatable_cartridge(cartridge);
    }
    if cartridge.get(4) == Some(&dali_amrn::v2::FORMAT_VERSION) {
        return inspect_isolation_cartridge(cartridge);
    }
    let header = dali_amrn::parse_header(cartridge)
        .map_err(|error| format!("invalid AMRN cartridge: {error:?}"))?;
    let expected_size = dali_amrn::HEADER_SIZE
        .checked_add(header.payload_size as usize)
        .ok_or_else(|| "cartridge size overflow".to_owned())?;
    if cartridge.len() != expected_size {
        return Err(format!(
            "invalid AMRN cartridge: file length is {}, expected {expected_size}",
            cartridge.len()
        ));
    }
    let parsed = dali_amrn::parse(cartridge)
        .map_err(|error| format!("invalid AMRN cartridge: {error:?}"))?;
    Ok(format!(
        "AMRN cartridge valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\npayload_size: {}\nload_address: 0x{:08X}\nexecution_offset: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
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

fn inspect_signed_cartridge(cartridge: &[u8]) -> Result<String, String> {
    let target_id = *cartridge
        .get(dali_amrn::v5::TARGET_ID_OFFSET)
        .ok_or_else(|| "invalid AMRN cartridge: truncated target identifier".to_owned())?;
    let target = dali_targets::find_by_amrn_target_id(target_id)
        .ok_or_else(|| format!("unsupported AMRN target identifier 0x{target_id:02X}"))?;
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| format!("target {} has no ABI v3 memory contract", target.name))?;
    let mut parsed = None;
    let mut last_error = dali_amrn::v5::Error::InvalidHeader;
    for slot in isolation.slots.iter().copied() {
        let contract = dali_amrn::v3::Contract {
            target_id,
            code_load_address: slot.code_origin,
            code_capacity: slot.code_length,
            data_load_address: slot.data_origin,
            data_capacity: slot.data_length,
        };
        match dali_amrn::v5::parse(cartridge, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN cartridge: {last_error:?}"))?;
    let metadata = parsed.header.metadata;
    Ok(format!(
        "AMRN cartridge valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncartridge_id: {:02X?}\ncartridge_version: {}.{}.{}\nminimum_kernel_version: {}.{}.{}\nrequired_services: 0x{:08X}\nslot_id: {}\ncode_size: {}\ndata_init_size: {}\nrelocation_count: {}\nabi_version: {}\ncartridge_crc32: 0x{:08X}\nsigning_key_id: {:02X?}",
        dali_amrn::v5::FORMAT_VERSION,
        parsed.header.image.target_id,
        dali_amrn::v5::HEADER_SIZE,
        metadata.cartridge_id,
        metadata.cartridge_version.major,
        metadata.cartridge_version.minor,
        metadata.cartridge_version.patch,
        metadata.minimum_kernel_version.major,
        metadata.minimum_kernel_version.minor,
        metadata.minimum_kernel_version.patch,
        metadata.required_services,
        metadata.slot_id,
        parsed.header.image.code_size,
        parsed.header.image.data_init_size,
        parsed.header.image.relocation_count,
        dali_amrn::v3::ABI_VERSION,
        parsed.header.cartridge_crc32,
        parsed.header.signature.key_id,
    ))
}

fn inspect_identity_cartridge(cartridge: &[u8]) -> Result<String, String> {
    let target_id = *cartridge
        .get(5)
        .ok_or_else(|| "invalid AMRN cartridge: truncated target identifier".to_owned())?;
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
        match dali_amrn::v4::parse(cartridge, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN cartridge: {last_error:?}"))?;
    let metadata = parsed.header.metadata;
    Ok(format!(
        "AMRN cartridge valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncartridge_id: {:02X?}\ncartridge_version: {}.{}.{}\nminimum_kernel_version: {}.{}.{}\nrequired_services: 0x{:08X}\nslot_id: {}\ncode_size: {}\ndata_init_size: {}\nrelocation_count: {}\nabi_version: {}\ncartridge_crc32: 0x{:08X}",
        dali_amrn::v4::FORMAT_VERSION,
        parsed.header.image.target_id,
        dali_amrn::v4::HEADER_SIZE,
        metadata.cartridge_id,
        metadata.cartridge_version.major,
        metadata.cartridge_version.minor,
        metadata.cartridge_version.patch,
        metadata.minimum_kernel_version.major,
        metadata.minimum_kernel_version.minor,
        metadata.minimum_kernel_version.patch,
        metadata.required_services,
        metadata.slot_id,
        parsed.header.image.code_size,
        parsed.header.image.data_init_size,
        parsed.header.image.relocation_count,
        dali_amrn::v4::ABI_VERSION,
        parsed.header.cartridge_crc32,
    ))
}

fn inspect_relocatable_cartridge(cartridge: &[u8]) -> Result<String, String> {
    let target_id = *cartridge
        .get(5)
        .ok_or_else(|| "invalid AMRN cartridge: truncated target identifier".to_owned())?;
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
        match dali_amrn::v3::parse(cartridge, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN cartridge: {last_error:?}"))?;
    Ok(format!(
        "AMRN cartridge valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncode_size: {}\ndata_init_size: {}\ndata_zero_size: {}\nstack_size: {}\nlinked_code_base: 0x{:08X}\nlinked_data_base: 0x{:08X}\ncode_load_address: 0x{:08X}\ndata_load_address: 0x{:08X}\nexecution_offset: {}\nrelocation_count: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
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

fn inspect_isolation_cartridge(cartridge: &[u8]) -> Result<String, String> {
    let target_id = *cartridge
        .get(5)
        .ok_or_else(|| "invalid AMRN cartridge: truncated target identifier".to_owned())?;
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
        match dali_amrn::v2::parse(cartridge, contract) {
            Ok(value) => {
                parsed = Some(value);
                break;
            }
            Err(error) => last_error = error,
        }
    }
    let parsed = parsed.ok_or_else(|| format!("invalid AMRN cartridge: {last_error:?}"))?;
    Ok(format!(
        "AMRN cartridge valid\nformat_version: {}\ntarget_id: 0x{:02X}\nheader_size: {}\ncode_size: {}\ndata_init_size: {}\ndata_zero_size: {}\nstack_size: {}\ncode_load_address: 0x{:08X}\ndata_load_address: 0x{:08X}\nexecution_offset: {}\nentry_address: 0x{:08X}\nabi_version: {}\ncrc32: 0x{:08X}",
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
    fn inspect_reports_contract_fields_for_valid_cartridge() {
        let cartridge = test_cartridge();
        let result = inspect_bytes(&cartridge);
        assert!(result.is_ok(), "cartridge should inspect");
        let report = result.unwrap_or_default();

        assert!(report.contains("AMRN cartridge valid"));
        assert!(report.contains("target_id: 0x02"));
        assert!(report.contains("abi_version: 2"));
        assert!(report.contains("payload_size: 4"));
    }

    #[test]
    fn inspect_rejects_trailing_bytes() {
        let mut cartridge = test_cartridge();
        cartridge.push(0);

        let result = inspect_bytes(&cartridge);
        assert!(result.is_err(), "trailing data must fail");
        let error = match result {
            Ok(_) => String::new(),
            Err(error) => error,
        };

        assert!(error.contains("file length"));
    }

    fn test_cartridge() -> Vec<u8> {
        let payload = [0x00, 0xBF, 0x00, 0xBF];
        let mut cartridge = vec![0; dali_amrn::HEADER_SIZE + payload.len()];
        let result = dali_amrn::encode_cartridge(&payload, 0, &mut cartridge);
        assert!(
            result.is_ok(),
            "test payload must produce a valid cartridge"
        );
        let written = result.unwrap_or_default();
        cartridge.truncate(written);
        cartridge
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
        let mut cartridge = vec![0; dali_amrn::v2::HEADER_SIZE + 8];
        let written = dali_amrn::v2::encode(image, contract, &mut cartridge)
            .expect("v2 test cartridge should encode");
        cartridge.truncate(written);
        let report = inspect_bytes(&cartridge).expect("v2 cartridge should inspect");
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
        let cartridge = identity_test_cartridge();
        let report = inspect_bytes(&cartridge).expect("v4 cartridge should inspect");
        assert!(report.contains("format_version: 4"));
        assert!(report.contains("cartridge_version: 1.2.3"));
        assert!(report.contains("slot_id: 1"));
    }

    #[test]
    fn inspect_reports_signed_fields() {
        let contract = dali_amrn::v3::Contract {
            target_id: 2,
            code_load_address: 0x2001_0000,
            code_capacity: 16 * 1024,
            data_load_address: 0x2001_4000,
            data_capacity: 16 * 1024,
        };
        let image = dali_amrn::v5::Image {
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
                cartridge_id: [2; 16],
                cartridge_version: dali_amrn::v4::Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
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
        let key_id = [7; dali_amrn::signature::KEY_ID_LENGTH];
        let private_key = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4f, 0xa4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1d, 0xe7, 0x5f, 0x60,
        ];
        let mut signed = vec![0; dali_amrn::v5::HEADER_SIZE + 4];
        let signed_size = dali_amrn::v5::encode_unsigned(image, contract, &mut signed)
            .expect("signed range should encode");
        signed.truncate(signed_size);
        let signature = dali_crypto::sign(&private_key, &signed);
        let mut cartridge = vec![0; signed_size + dali_amrn::v5::SIGNATURE_SIZE];
        let cartridge_size =
            dali_amrn::v5::append_signature(&signed, &key_id, &signature, &mut cartridge)
                .expect("signature envelope should append");
        cartridge.truncate(cartridge_size);

        let report = inspect_bytes(&cartridge).expect("v5 cartridge should inspect");
        assert!(report.contains("format_version: 5"));
        assert!(report.contains("signing_key_id: [07"));
    }

    #[test]
    fn inspect_rejects_a_corrupted_identity_cartridge() {
        let mut cartridge = identity_test_cartridge();
        cartridge[dali_amrn::v4::HEADER_SIZE] ^= 1;
        let error = inspect_bytes(&cartridge).expect_err("corrupted cartridge must be rejected");
        assert!(error.contains("CrcMismatch"));
    }

    fn identity_test_cartridge() -> Vec<u8> {
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
                cartridge_id: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                cartridge_version: dali_amrn::v4::Version {
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
        let mut cartridge = vec![0; dali_amrn::v4::HEADER_SIZE + 4];
        let size = dali_amrn::v4::encode(image, contract, &mut cartridge)
            .expect("v4 cartridge should encode");
        cartridge.truncate(size);
        cartridge
    }
}

#[cfg(test)]
#[path = "inspect_v3_tests.rs"]
mod v3_tests;
