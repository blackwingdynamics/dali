use std::{env, io};

const TARGET_PROFILE_ENV: &str = "DALI_TARGET_PROFILE";

fn main() -> Result<(), io::Error> {
    println!("cargo:rerun-if-env-changed={TARGET_PROFILE_ENV}");
    for target in dali_targets::SUPPORTED_TARGETS {
        println!(
            "cargo:rerun-if-env-changed={}",
            backend_feature_name(target.backend)
        );
    }
    let target = selected_target()?;
    if let Some(target) = target {
        let feature = backend_feature_name(target.backend);
        if env::var_os(&feature).is_none() {
            return Err(io::Error::other(format!(
                "target profile {:?} requires enabled backend feature {:?}",
                target.name, feature
            )));
        }
        println!(
            "cargo:rustc-env=DALI_SELECTED_TARGET_PROFILE={}",
            target.name
        );
    }
    forward_backend_linker_artifact()?;
    Ok(())
}

fn forward_backend_linker_artifact() -> Result<(), io::Error> {
    let roots: Vec<_> = env::vars_os()
        .filter(|(key, _)| {
            let key = key.to_string_lossy();
            key.starts_with("DEP_DALI_BOARD_") && key.ends_with("_ROOT")
        })
        .collect();
    match roots.as_slice() {
        [] => Ok(()),
        [(_, root)] => {
            println!("cargo:rustc-link-search={}", root.to_string_lossy());
            Ok(())
        }
        _ => Err(io::Error::other(
            "multiple backend linker artifacts are available; select one backend",
        )),
    }
}

fn selected_target() -> Result<Option<&'static dali_targets::TargetProfile>, io::Error> {
    if let Ok(name) = env::var(TARGET_PROFILE_ENV) {
        let target = dali_targets::find_target(&name).ok_or_else(|| {
            io::Error::other(format!(
                "DALI_TARGET_PROFILE={name:?} does not name an application-supported target"
            ))
        })?;
        return Ok(Some(target));
    }

    let mut selected = dali_targets::SUPPORTED_TARGETS
        .iter()
        .filter(|target| env::var_os(backend_feature_name(target.backend)).is_some());
    let Some(target) = selected.next() else {
        return Ok(None);
    };
    if selected.next().is_some() {
        return Err(io::Error::other(
            "multiple backend targets are enabled; set DALI_TARGET_PROFILE explicitly",
        ));
    }
    Ok(Some(target))
}

fn backend_feature_name(backend: &str) -> String {
    format!(
        "CARGO_FEATURE_{}",
        backend.replace('-', "_").to_ascii_uppercase()
    )
}
