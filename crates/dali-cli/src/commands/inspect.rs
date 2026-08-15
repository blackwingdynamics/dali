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
    let target = manifest_value(&contents, "target_profile")?;
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
}
