use std::{
    cmp::Ordering,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use dali_metadata::{
    BoundedText, BundleFile, BundleFileKind, Ed25519Verifier, MetadataRole, RootMetadata,
    Sha256Digest, SignedEnvelope, verify_role_signatures,
};
use sha2::{Digest, Sha256};

pub(super) const INPUT_FLAG: &str = "--input";
pub(super) const OUTPUT_FLAG: &str = "--output";
pub(super) const SIGNING_KEY_FLAG: &str = "--signing-key";
pub(super) const SIGNER_KEY_ID_FLAG: &str = "--signer-key-id";
pub(super) const TARGET_PROFILE_FLAG: &str = "--target-profile";
pub(super) const VERSION_FLAG: &str = "--version";
pub(super) const PACKAGE_ID_FLAG: &str = "--package-id";
pub(super) const MANIFEST_NAME: &str = "bundle.manifest";
pub(super) const METADATA_DIRECTORY: &str = "metadata";
pub(super) const DELEGATIONS_DIRECTORY: &str = "delegations";
pub(super) const PACKAGES_DIRECTORY: &str = "packages";

pub(super) fn required(arguments: &[String], flag: &str) -> Result<String, String> {
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

pub(super) fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an unsigned 64-bit integer"))
}

pub(super) fn parse_hex<const LENGTH: usize>(
    value: &str,
    field: &str,
) -> Result<[u8; LENGTH], String> {
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

pub(super) fn read_seed(path: &Path) -> Result<[u8; dali_crypto::PRIVATE_KEY_LENGTH], String> {
    parse_hex(
        fs::read_to_string(path)
            .map_err(|error| format!("cannot read signing key {}: {error}", path.display()))?
            .trim(),
        "signing key",
    )
}

pub(super) fn read_envelope(path: &Path, role: &str) -> Result<SignedEnvelope<'static>, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {role} metadata {}: {error}", path.display()))?;
    let bytes = Box::leak(bytes.into_boxed_slice());
    dali_metadata::parse_signed_envelope(bytes)
        .map_err(|error| format!("invalid {role} envelope: {error:?}"))
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

pub(super) fn bundle_file_path(root: &Path, file: BundleFile) -> Result<PathBuf, String> {
    let id = file
        .id
        .as_str()
        .ok_or_else(|| "bundle file ID is invalid".to_owned())?;
    Ok(match file.kind {
        BundleFileKind::Root => root.join(METADATA_DIRECTORY).join("root.json"),
        BundleFileKind::Timestamp => root.join(METADATA_DIRECTORY).join("timestamp.json"),
        BundleFileKind::Snapshot => root.join(METADATA_DIRECTORY).join("snapshot.json"),
        BundleFileKind::Targets => root.join(METADATA_DIRECTORY).join("targets.json"),
        BundleFileKind::Revocation => root.join(METADATA_DIRECTORY).join("revocations.json"),
        BundleFileKind::Delegation => root
            .join(METADATA_DIRECTORY)
            .join(DELEGATIONS_DIRECTORY)
            .join(format!("{id}.json")),
        BundleFileKind::Package => root.join(PACKAGES_DIRECTORY).join(format!("{id}.amrn")),
    })
}

pub(super) fn verify_bundle_signature(
    root: &RootMetadata,
    manifest: SignedEnvelope<'_>,
) -> Result<(), String> {
    let role = root
        .roles
        .iter()
        .take(usize::from(root.role_count))
        .find(|role| role.role == MetadataRole::Bundle)
        .ok_or_else(|| "root metadata does not declare bundle role".to_owned())?;
    verify_role_signatures(
        &Ed25519Verifier,
        manifest.signed,
        *role,
        &root.keys[..usize::from(root.key_count)],
        manifest.signatures,
    )
    .map_err(|error| format!("bundle manifest signature verification failed: {error:?}"))
}

pub(super) fn verify_bundle_files(
    root: &Path,
    bundle: &dali_metadata::BundleMetadata,
) -> Result<(), String> {
    for file in bundle.files.iter().take(usize::from(bundle.file_count)) {
        let path = bundle_file_path(root, *file)?;
        let bytes =
            fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let digest = Sha256::digest(&bytes);
        if bytes.len() != file.length as usize || digest[..] != file.sha256.0 {
            return Err(format!(
                "bundle reference mismatch for {}:{}",
                file.kind.as_str(),
                file.id.as_str().unwrap_or("<invalid>")
            ));
        }
    }
    Ok(())
}

pub(super) fn file_record(
    kind: BundleFileKind,
    id: &str,
    path: &Path,
) -> Result<BundleFile, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok(BundleFile {
        kind,
        id: BoundedText::new(id).map_err(|error| format!("invalid bundle file ID: {error:?}"))?,
        length: u32::try_from(bytes.len()).map_err(|_| "bundle file is too large".to_owned())?,
        sha256: Sha256Digest(Sha256::digest(bytes).into()),
    })
}

pub(super) fn bundle_order(left: BundleFile, right: BundleFile) -> Ordering {
    kind_order(left.kind)
        .cmp(&kind_order(right.kind))
        .then_with(|| {
            left.id
                .as_str()
                .unwrap_or("")
                .cmp(right.id.as_str().unwrap_or(""))
        })
}

fn kind_order(kind: BundleFileKind) -> u8 {
    match kind {
        BundleFileKind::Root => 0,
        BundleFileKind::Timestamp => 1,
        BundleFileKind::Snapshot => 2,
        BundleFileKind::Targets => 3,
        BundleFileKind::Revocation => 4,
        BundleFileKind::Delegation => 5,
        BundleFileKind::Package => 6,
    }
}

pub(super) fn file_count(files: &[BundleFile; dali_metadata::MAX_BUNDLE_FILES]) -> u16 {
    files.iter().take_while(|file| file.length != 0).count() as u16
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
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

fn usage() -> String {
    "usage: dali metadata bundle {generate|inspect|verify} ...".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dali_metadata::{BundleMetadata, MetadataHeader, MetadataRole};

    #[test]
    fn rejects_tampered_bundle_reference() {
        let root = std::env::temp_dir().join(format!(
            "dali-cli-bundle-reference-test-{}",
            std::process::id()
        ));
        let metadata = root.join(METADATA_DIRECTORY);
        fs::create_dir_all(&metadata).expect("create test metadata directory");
        let path = metadata.join("root.json");
        fs::write(&path, b"root metadata").expect("write test metadata");
        let file = file_record(BundleFileKind::Root, "root", &path).expect("record file");
        let mut files = [BundleFile::default(); dali_metadata::MAX_BUNDLE_FILES];
        files[0] = file;
        let bundle = BundleMetadata {
            header: MetadataHeader {
                role: MetadataRole::Bundle,
                version: 1,
                expires: 0,
            },
            target_profile: BoundedText::new("f405").expect("profile"),
            files,
            file_count: 1,
        };
        verify_bundle_files(&root, &bundle).expect("reference should initially match");

        fs::write(&path, b"tampered metadata").expect("tamper test metadata");
        let error = verify_bundle_files(&root, &bundle).expect_err("tampering must fail");
        assert!(error.contains("bundle reference mismatch for root:root"));
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn orders_bundle_files_by_role_then_id() {
        let root = BundleFile {
            kind: BundleFileKind::Root,
            id: BoundedText::new("root").expect("id"),
            length: 1,
            sha256: Sha256Digest([0; 32]),
        };
        let targets = BundleFile {
            kind: BundleFileKind::Targets,
            id: BoundedText::new("targets").expect("id"),
            length: 1,
            sha256: Sha256Digest([0; 32]),
        };
        assert_eq!(bundle_order(root, targets), Ordering::Less);
    }
}
