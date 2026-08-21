use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use dali_metadata::{
    BinaryEnvelope, KeyId, MAX_BUNDLE_BYTES, MAX_SIGNATURES, MetadataRole, RootMetadata, Signature,
    SignatureRecord, SignatureSet, encode_binary_envelope, parse_binary_envelope,
};

pub const INPUT_FLAG: &str = "--input";
pub const OUTPUT_FLAG: &str = "--output";
pub const ROOT_SIGNING_KEY_FLAG: &str = "--root-signing-key";
pub const ROOT_KEY_ID_FLAG: &str = "--root-key-id";
pub const BUNDLE_SIGNING_KEY_FLAG: &str = "--bundle-signing-key";
pub const BUNDLE_KEY_ID_FLAG: &str = "--bundle-key-id";
pub const SIGNING_KEY_FLAG: &str = "--signing-key";
pub const DEVELOPER_ID_FLAG: &str = "--developer-id";
pub const DEVELOPER_KEY_ID_FLAG: &str = "--developer-key-id";
pub const DEVELOPER_PUBLIC_KEY_FLAG: &str = "--developer-public-key";
pub const DELEGATION_ID_FLAG: &str = "--delegation-id";
pub const NAMESPACE_FLAG: &str = "--namespace";
pub const TARGET_FLAG: &str = "--target";
pub const ABI_FLAG: &str = "--abi";
pub const PACKAGE_FLAG: &str = "--package";
pub const MANIFEST_FLAG: &str = "--manifest";
pub const TARGET_PROFILE_FLAG: &str = "--target-profile";
pub const VERSION_FLAG: &str = "--version";
pub const METADATA_DIRECTORY: &str = "metadata";
pub const DELEGATIONS_DIRECTORY: &str = "delegat";
pub const PACKAGES_DIRECTORY: &str = "packages";

pub fn required(arguments: &[String], flag: &str) -> Result<String, String> {
    let position = arguments
        .iter()
        .position(|argument| argument == flag)
        .ok_or_else(usage)?;
    arguments
        .get(position + 1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(usage)
}

pub fn parse_hex<const LENGTH: usize>(value: &str, field: &str) -> Result<[u8; LENGTH], String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != LENGTH * 2 {
        return Err(format!(
            "{field} must contain exactly {} hexadecimal digits",
            LENGTH * 2
        ));
    }
    let mut output = [0; LENGTH];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| format!("{field} contains invalid hexadecimal data"))?;
    }
    Ok(output)
}

pub fn parse_u16(value: &str, field: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an unsigned 16-bit integer"))
}

pub fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an unsigned 64-bit integer"))
}

pub fn read_seed(path: &Path) -> Result<[u8; dali_crypto::PRIVATE_KEY_LENGTH], String> {
    parse_hex(
        fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?
            .trim(),
        "signing key",
    )
}

pub fn read_binary(
    path: &Path,
    role: MetadataRole,
) -> Result<(Vec<u8>, BinaryEnvelope<'static>), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let leaked = Box::leak(bytes.clone().into_boxed_slice());
    let envelope = parse_binary_envelope(leaked)
        .map_err(|error| format!("invalid {} metadata: {error:?}", role.as_str()))?;
    if envelope.role != role {
        return Err(format!(
            "{} metadata has role {}",
            role.as_str(),
            envelope.role.as_str()
        ));
    }
    Ok((bytes, envelope))
}

pub fn sign_binary(
    role: MetadataRole,
    body: &[u8],
    seed: &[u8; 32],
    key_id: KeyId,
) -> Result<Vec<u8>, String> {
    let signature = Signature(dali_crypto::sign(seed, body));
    let mut records = [SignatureRecord::default(); MAX_SIGNATURES];
    records[0] = SignatureRecord { key_id, signature };
    let mut output = vec![0; MAX_BUNDLE_BYTES];
    let length =
        encode_binary_envelope(role, body, SignatureSet { records, count: 1 }, &mut output)
            .map_err(|error| format!("cannot encode {} envelope: {error:?}", role.as_str()))?;
    output.truncate(length);
    Ok(output)
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

pub fn write_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("dmb.tmp");
    fs::write(&temporary, bytes)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}

pub fn role_key(root: &RootMetadata, role: MetadataRole) -> Result<KeyId, String> {
    root.roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|item| item.role == role)
        .and_then(|item| item.keys.first().copied())
        .ok_or_else(|| {
            format!(
                "root metadata does not declare a signer for {}",
                role.as_str()
            )
        })
}

pub fn root_path(root: &Path) -> PathBuf {
    root.join(METADATA_DIRECTORY).join("root.dmb")
}
pub fn timestamp_path(root: &Path) -> PathBuf {
    root.join(METADATA_DIRECTORY).join("timestamp.dmb")
}
pub fn snapshot_path(root: &Path) -> PathBuf {
    root.join(METADATA_DIRECTORY).join("snapshot.dmb")
}
pub fn targets_path(root: &Path) -> PathBuf {
    root.join(METADATA_DIRECTORY).join("targets.dmb")
}
pub fn revocations_path(root: &Path) -> PathBuf {
    root.join(METADATA_DIRECTORY).join("revocations.dmb")
}
pub fn delegation_path(root: &Path, id: &str) -> PathBuf {
    root.join(METADATA_DIRECTORY)
        .join(DELEGATIONS_DIRECTORY)
        .join(format!("{id}.dmb"))
}

pub fn bounded<const N: usize>(
    value: &str,
    field: &str,
) -> Result<dali_metadata::BoundedText<N>, String> {
    dali_metadata::BoundedText::new(value).map_err(|error| format!("{field} is invalid: {error:?}"))
}

pub fn random_key_id() -> Result<KeyId, String> {
    let mut bytes = [0; dali_metadata::KEY_ID_LENGTH];
    getrandom::fill(&mut bytes).map_err(|error| format!("cannot obtain OS randomness: {error}"))?;
    Ok(KeyId(bytes))
}

pub fn usage() -> String {
    "usage: dali metadata repository {init|add-developer|register-package|publish} ...".to_owned()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                HEX[(byte >> 4) as usize] as char,
                HEX[(byte & 0x0f) as usize] as char,
            ]
        })
        .collect()
}
