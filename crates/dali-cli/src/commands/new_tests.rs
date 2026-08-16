use super::{create_project, initialize_project, render, validate_name};
use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn accepts_cargo_style_application_names() {
    assert!(validate_name("telemetry_app").is_ok());
    assert!(validate_name("telemetry-app").is_ok());
}

#[test]
fn rejects_paths_and_empty_names() {
    assert!(validate_name("").is_err());
    assert!(validate_name("../telemetry").is_err());
    assert!(validate_name("telemetry/app").is_err());
    assert!(validate_name("telemetry app").is_err());
}

#[test]
fn renders_application_name_and_sdk_path() {
    let rendered = render(
        "name={{ application_name }} sdk={{ sdk_path }} target={{ target_profile }}",
        "demo",
        "../sdk",
        "test-target",
    );
    assert_eq!(rendered, "name=demo sdk=../sdk target=test-target");
}

#[test]
fn creates_the_documented_project_files() -> Result<(), Box<dyn std::error::Error>> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = env::temp_dir().join(format!("dali-cli-app-new-{suffix}"));
    fs::create_dir(&root)?;
    create_project(&root, "demo", "../crates/dali-sdk")?;
    assert!(root.join("demo/Cargo.toml").is_file());
    assert!(root.join("demo/dali.toml").is_file());
    assert!(root.join("demo/memory.v3.x").is_file());
    assert!(root.join("demo/src/main.rs").is_file());
    let linker = fs::read_to_string(root.join("demo/memory.v3.x"))?;
    let target = dali_targets::SUPPORTED_TARGETS[0];
    let isolation = target.memory.isolation.expect("isolation metadata");
    assert!(linker.contains(&format!("ORIGIN = 0x{:08X}", isolation.code_origin)));
    assert!(linker.contains(&format!("ORIGIN = 0x{:08X}", isolation.data_origin)));
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn initializes_only_an_empty_managed_project() -> Result<(), Box<dyn std::error::Error>> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = env::temp_dir().join(format!("dali-cli-app-init-{suffix}"));
    fs::create_dir(&root)?;
    fs::write(root.join("README.md"), "user content")?;
    initialize_project(&root, "demo", "../crates/dali-sdk")?;
    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join("README.md").is_file());
    let result = initialize_project(&root, "demo", "../crates/dali-sdk");
    assert!(result.is_err());
    fs::remove_dir_all(root)?;
    Ok(())
}
