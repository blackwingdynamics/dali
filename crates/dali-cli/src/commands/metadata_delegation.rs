//! Authority-side developer delegation artifact generation.

use std::{fs, fs::OpenOptions, io::Write, path::Path};

use dali_metadata::{
    BoundedText, DelegationMetadata, KeyId, MAX_DELEGATION_ABIS, MAX_DELEGATION_BYTES,
    MAX_DELEGATION_SCOPES, MAX_DELEGATION_TARGETS, MAX_ENVELOPE_BYTES, MetadataHeader,
    MetadataRole, PublicKey, Signature, SignatureRecord, SignatureSet, encode_delegation_signed,
    encode_signed_envelope,
};

const CREATE_COMMAND: &str = "create";
const INSPECT_COMMAND: &str = "inspect";
const INPUT_FLAG: &str = "--input";
const OUTPUT_FLAG: &str = "--output";
const SIGNING_KEY_FLAG: &str = "--signing-key";
const SIGNER_KEY_ID_FLAG: &str = "--signer-key-id";
const SIGNER_PUBLIC_KEY_FLAG: &str = "--signer-public-key";
const DEVELOPER_ID_FLAG: &str = "--developer-id";
const DEVELOPER_KEY_ID_FLAG: &str = "--developer-key-id";
const DEVELOPER_PUBLIC_KEY_FLAG: &str = "--developer-public-key";
const NAMESPACE_FLAG: &str = "--namespace";
const TARGET_FLAG: &str = "--target";
const ABI_FLAG: &str = "--abi";
const VERSION_FLAG: &str = "--version";
const EXPIRES_FLAG: &str = "--expires";
const NOT_BEFORE_FLAG: &str = "--not-before";
const NOT_AFTER_FLAG: &str = "--not-after";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(2).map(String::as_str) {
        Some(CREATE_COMMAND) => create(arguments),
        Some(INSPECT_COMMAND) => inspect(arguments),
        _ => Err(usage()),
    }
}

fn create(arguments: &[String]) -> Result<(), String> {
    let output = required(arguments, OUTPUT_FLAG)?;
    let signing_key_path = required(arguments, SIGNING_KEY_FLAG)?;
    let signer_key_id = KeyId(parse_hex::<{ dali_metadata::KEY_ID_LENGTH }>(
        &required(arguments, SIGNER_KEY_ID_FLAG)?,
        SIGNER_KEY_ID_FLAG,
    )?);
    let developer_id = bounded::<{ dali_metadata::MAX_DEVELOPER_ID_BYTES }>(
        &required(arguments, DEVELOPER_ID_FLAG)?,
        DEVELOPER_ID_FLAG,
    )?;
    let developer_key_id = KeyId(parse_hex::<{ dali_metadata::KEY_ID_LENGTH }>(
        &required(arguments, DEVELOPER_KEY_ID_FLAG)?,
        DEVELOPER_KEY_ID_FLAG,
    )?);
    let developer_public_key = PublicKey(parse_hex::<{ dali_metadata::PUBLIC_KEY_LENGTH }>(
        &required(arguments, DEVELOPER_PUBLIC_KEY_FLAG)?,
        DEVELOPER_PUBLIC_KEY_FLAG,
    )?);
    let namespace = bounded::<{ dali_metadata::MAX_NAMESPACE_BYTES }>(
        &required(arguments, NAMESPACE_FLAG)?,
        NAMESPACE_FLAG,
    )?;
    let target = bounded::<{ dali_metadata::MAX_TARGET_PROFILE_BYTES }>(
        &required(arguments, TARGET_FLAG)?,
        TARGET_FLAG,
    )?;
    let abi = parse_u16(&required(arguments, ABI_FLAG)?, ABI_FLAG)?;
    let version = parse_u64(&required(arguments, VERSION_FLAG)?, VERSION_FLAG)?;
    let expires = optional_u64(arguments, EXPIRES_FLAG)?.unwrap_or(0);
    let not_before = optional_u64(arguments, NOT_BEFORE_FLAG)?.unwrap_or(0);
    let not_after = optional_u64(arguments, NOT_AFTER_FLAG)?.unwrap_or(0);

    let mut namespaces = [BoundedText::default(); MAX_DELEGATION_SCOPES];
    namespaces[0] = namespace;
    let mut targets = [BoundedText::default(); MAX_DELEGATION_TARGETS];
    targets[0] = target;
    let mut abis = [0; MAX_DELEGATION_ABIS];
    abis[0] = abi;
    let metadata = DelegationMetadata {
        header: MetadataHeader {
            role: MetadataRole::Delegation,
            version,
            expires,
        },
        developer_id,
        key_id: developer_key_id,
        public_key: developer_public_key,
        allowed_namespaces: namespaces,
        namespace_count: 1,
        allowed_targets: targets,
        target_count: 1,
        allowed_abis: abis,
        abi_count: 1,
        not_before,
        not_after,
    };
    let seed = read_seed(Path::new(&signing_key_path))?;
    let mut signed = [0; MAX_DELEGATION_BYTES];
    let signed_length = encode_delegation_signed(&mut signed, metadata)
        .map_err(|error| format!("cannot encode delegation: {error:?}"))?;
    let signature = dali_crypto::sign(&seed, &signed[..signed_length]);
    let mut records = [SignatureRecord::default(); dali_metadata::MAX_SIGNATURES];
    records[0] = SignatureRecord {
        key_id: signer_key_id,
        signature: Signature(signature),
    };
    let mut envelope = [0; MAX_ENVELOPE_BYTES];
    let envelope_length = encode_signed_envelope(
        &mut envelope,
        &signed[..signed_length],
        SignatureSet { records, count: 1 },
    )
    .map_err(|error| format!("cannot encode delegation envelope: {error:?}"))?;
    write_new(Path::new(&output), &envelope[..envelope_length])?;
    println!("Created signed delegation metadata: {output}");
    Ok(())
}

fn inspect(arguments: &[String]) -> Result<(), String> {
    let input = required(arguments, INPUT_FLAG)?;
    let signer_public_key = PublicKey(parse_hex::<{ dali_metadata::PUBLIC_KEY_LENGTH }>(
        &required(arguments, SIGNER_PUBLIC_KEY_FLAG)?,
        SIGNER_PUBLIC_KEY_FLAG,
    )?);
    let bytes = fs::read(&input).map_err(|error| format!("cannot read {input}: {error}"))?;
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err("delegation envelope exceeds the bounded metadata limit".to_owned());
    }
    let envelope = dali_metadata::parse_signed_envelope(&bytes)
        .map_err(|error| format!("invalid delegation envelope: {error:?}"))?;
    if envelope.signatures.count != 1 {
        return Err("delegation inspection requires exactly one signature".to_owned());
    }
    let delegation = dali_metadata::parse_delegation_signed(envelope.signed)
        .map_err(|error| format!("invalid delegation body: {error:?}"))?;
    let record = envelope.signatures.records[0];
    dali_crypto::verify(&signer_public_key.0, envelope.signed, &record.signature.0)
        .map_err(|error| format!("delegation signature verification failed: {error:?}"))?;
    println!("Delegation metadata valid");
    println!(
        "developer_id: {}",
        delegation.developer_id.as_str().unwrap_or("<invalid>")
    );
    println!("developer_key_id: {}", hex_encode(&delegation.key_id.0));
    println!("signer_key_id: {}", hex_encode(&record.key_id.0));
    println!("version: {}", delegation.header.version);
    println!("signature: verified");
    Ok(())
}

fn required(arguments: &[String], flag: &str) -> Result<String, String> {
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

fn optional_u64(arguments: &[String], flag: &str) -> Result<Option<u64>, String> {
    let Some(position) = arguments.iter().position(|argument| argument == flag) else {
        return Ok(None);
    };
    let value = arguments.get(position + 1).ok_or_else(usage)?;
    parse_u64(value, flag).map(Some)
}

fn bounded<const CAPACITY: usize>(
    value: &str,
    field: &str,
) -> Result<BoundedText<CAPACITY>, String> {
    BoundedText::new(value).map_err(|error| format!("{field} is invalid: {error:?}"))
}

fn parse_u16(value: &str, field: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an unsigned 16-bit integer"))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an unsigned 64-bit integer"))
}

fn parse_hex<const LENGTH: usize>(value: &str, field: &str) -> Result<[u8; LENGTH], String> {
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

fn read_seed(path: &Path) -> Result<[u8; dali_crypto::PRIVATE_KEY_LENGTH], String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("cannot read signing key {}: {error}", path.display()))?;
    parse_hex(value.trim(), "signing key")
}

fn write_new(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(contents)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn usage() -> String {
    "usage: dali metadata delegation {create|inspect} ...".to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parses_fixed_width_hex_values() {
        assert_eq!(parse_hex::<2>("0x01af", "value"), Ok([1, 0xaf]));
    }

    #[test]
    fn rejects_hex_values_with_the_wrong_width() {
        assert!(parse_hex::<2>("01", "value").is_err());
    }
}
