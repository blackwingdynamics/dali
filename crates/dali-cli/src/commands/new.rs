use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

#[path = "app_linker.rs"]
mod app_linker;

const SDK_CRATE_RELATIVE_PATH: &str = "crates/dali-sdk";
const SDK_PATH_FLAG: &str = "--sdk-path";
const APPLICATION_NAME_TOKEN: &str = "{{ application_name }}";
const SDK_PATH_TOKEN: &str = "{{ sdk_path }}";
const TARGET_PROFILE_TOKEN: &str = "{{ target_profile }}";
const ABI_VERSION_TOKEN: &str = "{{ abi_version }}";

const CARGO_TEMPLATE: &str = include_str!("../../templates/app/Cargo.toml.template");
const MANIFEST_TEMPLATE: &str = include_str!("../../templates/app/dali.toml.template");
const BUILD_TEMPLATE: &str = include_str!("../../templates/app/build.rs.template");
const MEMORY_TEMPLATE: &str = include_str!("../../templates/app/memory.x.template");
const V3_MEMORY_TEMPLATE: &str = include_str!("../../templates/app/memory.v3.x.template");
const LIB_TEMPLATE: &str = include_str!("../../templates/app/lib.rs.template");
const MAIN_TEMPLATE: &str = include_str!("../../templates/app/main.rs.template");
const CARGO_CONFIG_TEMPLATE: &str = include_str!("../../templates/app/config.toml.template");

struct Template {
    relative_path: &'static str,
    contents: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        relative_path: "Cargo.toml",
        contents: CARGO_TEMPLATE,
    },
    Template {
        relative_path: "dali.toml",
        contents: MANIFEST_TEMPLATE,
    },
    Template {
        relative_path: "build.rs",
        contents: BUILD_TEMPLATE,
    },
    Template {
        relative_path: app_linker::V2_MEMORY_FILE,
        contents: MEMORY_TEMPLATE,
    },
    Template {
        relative_path: app_linker::V3_MEMORY_FILE,
        contents: V3_MEMORY_TEMPLATE,
    },
    Template {
        relative_path: "src/lib.rs",
        contents: LIB_TEMPLATE,
    },
    Template {
        relative_path: "src/main.rs",
        contents: MAIN_TEMPLATE,
    },
    Template {
        relative_path: ".cargo/config.toml",
        contents: CARGO_CONFIG_TEMPLATE,
    },
];

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let name = arguments.get(2).ok_or_else(usage)?;
    validate_name(name)?;
    let sdk_override = match arguments.len() {
        3 => None,
        5 if arguments.get(3).map(String::as_str) == Some(SDK_PATH_FLAG) => {
            Some(PathBuf::from(&arguments[4]))
        }
        _ => return Err(usage()),
    };

    let parent = env::current_dir()
        .map_err(|error| format!("cannot determine current directory: {error}"))?;
    let sdk_directory = resolve_sdk_directory(&parent, sdk_override.as_deref())?;
    let project_directory = parent.join(name);
    let sdk_path = relative_path(&project_directory, &sdk_directory)?;
    create_project(&parent, name, &sdk_path)?;
    println!(
        "Created Dali application `{name}` at {}",
        project_directory.display()
    );
    Ok(())
}

pub(super) fn validate_name(name: &str) -> Result<(), String> {
    let is_single_component = Path::new(name)
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    let valid_characters = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'));
    if name.is_empty() || !is_single_component || !valid_characters {
        return Err(format!("invalid application name `{name}`"));
    }
    Ok(())
}

pub(super) fn resolve_sdk_directory(
    start: &Path,
    override_path: Option<&Path>,
) -> Result<PathBuf, String> {
    let sdk_directory = if let Some(path) = override_path {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            start.join(path)
        };
        fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve {SDK_PATH_FLAG} path: {error}"))?
    } else {
        find_workspace_root(start).ok_or_else(|| {
            format!(
                "Dali workspace root was not found; pass {SDK_PATH_FLAG} <path> for an external project"
            )
        })?
    };
    validate_sdk_directory(&sdk_directory)?;
    Ok(sdk_directory)
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    for candidate in start.ancestors() {
        let workspace_manifest = candidate.join("Cargo.toml");
        let sdk_manifest = candidate.join(SDK_CRATE_RELATIVE_PATH).join("Cargo.toml");
        if workspace_manifest.is_file() && sdk_manifest.is_file() {
            return fs::canonicalize(candidate.join(SDK_CRATE_RELATIVE_PATH)).ok();
        }
    }
    None
}

fn validate_sdk_directory(sdk_directory: &Path) -> Result<(), String> {
    let manifest = sdk_directory.join("Cargo.toml");
    let contents = fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read SDK manifest {}: {error}", manifest.display()))?;
    let has_dali_package_name = contents
        .lines()
        .any(|line| line.trim() == "name = \"dali\"");
    if !has_dali_package_name {
        return Err(format!(
            "{SDK_PATH_FLAG} must point to a Cargo crate named `dali`: {}",
            sdk_directory.display()
        ));
    }
    Ok(())
}

pub(super) fn relative_path(from: &Path, to: &Path) -> Result<String, String> {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common_length = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(from_component, to_component)| from_component == to_component)
        .count();
    if common_length == 0 {
        return Err("application and Dali workspace must share a filesystem root".to_owned());
    }

    let mut path = PathBuf::new();
    for _ in common_length..from_components.len() {
        path.push("..");
    }
    for component in &to_components[common_length..] {
        path.push(component.as_os_str());
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn create_project(parent: &Path, name: &str, sdk_path: &str) -> Result<(), String> {
    let project_directory = parent.join(name);
    if project_directory.exists() {
        return Err(format!(
            "refusing to overwrite existing path {}",
            project_directory.display()
        ));
    }
    let source_directory = project_directory.join("src");
    fs::create_dir_all(&source_directory).map_err(|error| {
        format!(
            "cannot create application directory {}: {error}",
            project_directory.display()
        )
    })?;

    for template in TEMPLATES {
        let destination = project_directory.join(template.relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let contents = render_template(template, name, sdk_path)?;
        fs::write(&destination, contents)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }
    Ok(())
}

pub(super) fn initialize_project(
    project_directory: &Path,
    name: &str,
    sdk_path: &str,
) -> Result<(), String> {
    for template in TEMPLATES {
        let destination = project_directory.join(template.relative_path);
        if destination.exists() {
            return Err(format!(
                "refusing to overwrite existing managed file {}",
                destination.display()
            ));
        }
    }

    for template in TEMPLATES {
        let destination = project_directory.join(template.relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let contents = render_template(template, name, sdk_path)?;
        fs::write(&destination, contents)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }
    Ok(())
}

fn render_template(template: &Template, name: &str, sdk_path: &str) -> Result<String, String> {
    if template.relative_path == app_linker::V3_MEMORY_FILE {
        return app_linker::render_isolation_memory_script(template.contents);
    }
    if template.relative_path == app_linker::V2_MEMORY_FILE {
        return app_linker::render_legacy_memory_script(template.contents);
    }
    let target = app_linker::default_target()?;
    Ok(render(
        template.contents,
        name,
        sdk_path,
        target.name,
        target.abi_version,
    ))
}

fn render(
    template: &str,
    name: &str,
    sdk_path: &str,
    target_profile: &str,
    abi_version: u8,
) -> String {
    template
        .replace(APPLICATION_NAME_TOKEN, name)
        .replace(SDK_PATH_TOKEN, sdk_path)
        .replace(TARGET_PROFILE_TOKEN, target_profile)
        .replace(ABI_VERSION_TOKEN, &abi_version.to_string())
}

fn usage() -> String {
    "usage:\n  dali app new <name> [--sdk-path <path>]".to_owned()
}

#[cfg(test)]
#[path = "new_tests.rs"]
mod tests;
