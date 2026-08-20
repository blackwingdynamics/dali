use std::{env, path::PathBuf};

use super::new::{initialize_project, relative_path, resolve_sdk_directory, validate_name};

const SDK_PATH_FLAG: &str = "--sdk-path";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let sdk_override = match arguments.len() {
        2 => None,
        4 if arguments.get(2).map(String::as_str) == Some(SDK_PATH_FLAG) => {
            Some(PathBuf::from(&arguments[3]))
        }
        _ => return Err(usage()),
    };
    let project_directory = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let name = project_directory
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "current directory has no valid application name".to_owned())?;
    validate_name(name)?;
    let sdk_directory = resolve_sdk_directory(&project_directory, sdk_override.as_deref())?;
    let sdk_path = relative_path(&project_directory, &sdk_directory)?;
    initialize_project(&project_directory, name, &sdk_path)?;
    println!(
        "Initialized Dali application `{name}` in {}",
        project_directory.display()
    );
    Ok(())
}

fn usage() -> String {
    "usage:\n  dali app init [--sdk-path <path>]".to_owned()
}
