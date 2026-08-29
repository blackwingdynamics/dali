//! Repository trust-store bootstrap and publication commands.

mod add_developer;
mod common;
mod init;
mod manifest;
mod publish;
mod register_cartridge;

const INIT_COMMAND: &str = "init";
const ADD_DEVELOPER_COMMAND: &str = "add-developer";
const PUBLISH_COMMAND: &str = "publish";
const REGISTER_CARTRIDGE_COMMAND: &str = "register-cartridge";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(2).map(String::as_str) {
        Some(INIT_COMMAND) => init::run(arguments),
        Some(ADD_DEVELOPER_COMMAND) => add_developer::run(arguments),
        Some(PUBLISH_COMMAND) => publish::run(arguments),
        Some(REGISTER_CARTRIDGE_COMMAND) => register_cartridge::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: dali metadata repository {init|add-developer|register-cartridge|publish} ...".to_owned()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use dali_metadata::{
        MetadataRole, parse_binary_delegation_body, parse_binary_envelope, parse_binary_root_body,
        parse_binary_targets_body,
    };

    use super::*;

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!("dali-repository-bootstrap-{}", std::process::id()))
    }

    fn seed(path: &PathBuf, value: u8) {
        fs::write(
            path,
            format!("{}\n", super::common::hex_encode(&[value; 32])),
        )
        .expect("write test seed");
    }

    #[test]
    fn initializes_binary_v2_repository_and_adds_developer() {
        let root = test_root();
        let _ = fs::remove_dir_all(&root);
        let root_seed = root.with_extension("root.seed");
        let bundle_seed = root.with_extension("bundle.seed");
        seed(&root_seed, 1);
        seed(&bundle_seed, 2);
        let repository = root.join("repository");
        init::run(&[
            "metadata".into(),
            "repository".into(),
            "init".into(),
            "--output".into(),
            repository.display().to_string(),
            "--root-signing-key".into(),
            root_seed.display().to_string(),
            "--root-key-id".into(),
            "01010101010101010101010101010101".into(),
            "--bundle-signing-key".into(),
            bundle_seed.display().to_string(),
            "--bundle-key-id".into(),
            "02020202020202020202020202020202".into(),
        ])
        .expect("initialize repository");
        let root_bytes = fs::read(common::root_path(&repository)).expect("read root");
        let root_envelope = parse_binary_envelope(&root_bytes).expect("parse root envelope");
        let root_metadata = parse_binary_root_body(root_envelope.body).expect("parse root body");
        assert_eq!(root_envelope.role, MetadataRole::Root);
        assert_eq!(root_metadata.role_count, 7);

        let developer_key = dali_crypto::public_key_from_seed(&[3; 32]);
        add_developer::run(&[
            "metadata".into(),
            "repository".into(),
            "add-developer".into(),
            "--input".into(),
            repository.display().to_string(),
            "--signing-key".into(),
            root_seed.display().to_string(),
            "--developer-id".into(),
            "developer-one".into(),
            "--developer-key-id".into(),
            "03030303030303030303030303030303".into(),
            "--developer-public-key".into(),
            super::common::hex_encode(&developer_key),
            "--delegation-id".into(),
            "developer-one-delegation".into(),
            "--namespace".into(),
            "developer-one".into(),
            "--target".into(),
            "f405".into(),
            "--abi".into(),
            "3".into(),
        ])
        .expect("add developer");
        let targets_bytes = fs::read(common::targets_path(&repository)).expect("read targets");
        let targets_envelope =
            parse_binary_envelope(&targets_bytes).expect("parse targets envelope");
        let targets = parse_binary_targets_body(targets_envelope.body).expect("parse targets body");
        assert_eq!(targets.delegation_count, 1);
        let delegation_path = common::delegation_path(&repository, "developer-one-delegation");
        let delegation_bytes = fs::read(delegation_path).expect("read delegation");
        let delegation_envelope =
            parse_binary_envelope(&delegation_bytes).expect("parse delegation envelope");
        let delegation =
            parse_binary_delegation_body(delegation_envelope.body).expect("parse delegation body");
        assert_eq!(delegation.developer_id.as_str(), Some("developer-one"));
        fs::write(repository.join("amrns/test.amrn"), b"test cartridge")
            .expect("write cartridge fixture");
        publish::run(&[
            "metadata".into(),
            "repository".into(),
            "publish".into(),
            "--input".into(),
            repository.display().to_string(),
            "--root-signing-key".into(),
            root_seed.display().to_string(),
            "--bundle-signing-key".into(),
            bundle_seed.display().to_string(),
            "--bundle-key-id".into(),
            "02020202020202020202020202020202".into(),
            "--target-profile".into(),
            "f405".into(),
            "--version".into(),
            "1".into(),
        ])
        .expect("publish repository");
        assert!(repository.join("bundle.manifest").is_file());
        fs::remove_file(root_seed).expect("remove root seed");
        fs::remove_file(bundle_seed).expect("remove bundle seed");
        fs::remove_dir_all(root).expect("remove repository test");
    }
}
