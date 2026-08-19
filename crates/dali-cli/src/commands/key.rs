//! Host-side Ed25519 trust-anchor generation.

use std::{fs::OpenOptions, io::Write, path::Path};

const GENERATE_COMMAND: &str = "generate";
const PRIVATE_OUTPUT_FLAG: &str = "--private-output";
const PUBLIC_OUTPUT_FLAG: &str = "--public-output";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 6 || arguments.get(1).map(String::as_str) != Some(GENERATE_COMMAND) {
        return Err(usage());
    }
    let private_path = required_flag(arguments, PRIVATE_OUTPUT_FLAG)?;
    let public_path = required_flag(arguments, PUBLIC_OUTPUT_FLAG)?;
    let mut seed = [0; dali_crypto::PRIVATE_KEY_LENGTH];
    let mut key_id = [0; dali_crypto::KEY_ID_LENGTH];
    getrandom::fill(&mut seed).map_err(|error| format!("cannot obtain OS randomness: {error}"))?;
    getrandom::fill(&mut key_id)
        .map_err(|error| format!("cannot obtain OS randomness: {error}"))?;
    let public_key = dali_crypto::public_key_from_seed(&seed);
    create_file(
        Path::new(&public_path),
        &public_record(&key_id, &public_key),
        false,
    )?;
    create_file(Path::new(&private_path), &hex_record(&seed), true)?;
    println!("Created public trust-anchor record: {public_path}");
    println!("Created private signing seed file: {private_path}");
    println!("Keep the private seed outside the repository and CI logs.");
    Ok(())
}

fn required_flag(arguments: &[String], flag: &str) -> Result<String, String> {
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

fn create_file(path: &Path, contents: &str, private: bool) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if private { 0o600 } else { 0o644 });
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn public_record(
    key_id: &[u8; dali_crypto::KEY_ID_LENGTH],
    public_key: &[u8; dali_crypto::PUBLIC_KEY_LENGTH],
) -> String {
    format!(
        "{{ key_id = \"{}\", public_key = \"{}\" }}\n",
        hex_encode(key_id),
        hex_encode(public_key)
    )
}

fn hex_record(seed: &[u8; dali_crypto::PRIVATE_KEY_LENGTH]) -> String {
    format!("{}\n", hex_encode(seed))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0F) as usize] as char);
    }
    output
}

fn usage() -> String {
    "usage: dali key generate --private-output <seed-file> --public-output <trust-anchor-fragment>"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{hex_encode, public_record};

    #[test]
    fn renders_manifest_ready_public_metadata_without_private_material() {
        let record = public_record(&[0xAB; 16], &[0xCD; 32]);
        assert_eq!(
            record,
            "{ key_id = \"ABABABABABABABABABABABABABABABAB\", public_key = \"CDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCD\" }\n"
        );
        assert_eq!(hex_encode(&[0x01, 0xAF]), "01AF");
    }
}
