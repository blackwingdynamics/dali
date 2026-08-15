use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

const SDK_CRATE_RELATIVE_PATH: &str = "crates/dali-sdk";
const SDK_PATH_FLAG: &str = "--sdk-path";
const APPLICATION_NAME_TOKEN: &str = "{{ application_name }}";
const SDK_PATH_TOKEN: &str = "{{ sdk_path }}";

const CARGO_TEMPLATE: &str = include_str!("../../templates/app/Cargo.toml.template");
const MANIFEST_TEMPLATE: &str = include_str!("../../templates/app/dali.toml.template");
const BUILD_TEMPLATE: &str = include_str!("../../templates/app/build.rs.template");
const MEMORY_TEMPLATE: &str = include_str!("../../templates/app/memory.x.template");
const LIB_TEMPLATE: &str = include_str!("../../templates/app/lib.rs.template");
const MAIN_TEMPLATE: &str = include_str!("../../templates/app/main.rs.template");

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
        relative_path: "memory.x",
        contents: MEMORY_TEMPLATE,
    },
    Template {
        relative_path: "src/lib.rs",
        contents: LIB_TEMPLATE,
    },
    Template {
        relative_path: "src/main.rs",
        contents: MAIN_TEMPLATE,
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

fn validate_name(name: &str) -> Result<(), String> {
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

fn resolve_sdk_directory(start: &Path, override_path: Option<&Path>) -> Result<PathBuf, String> {
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

fn relative_path(from: &Path, to: &Path) -> Result<String, String> {
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
        let contents = render(template.contents, name, sdk_path);
        fs::write(&destination, contents)
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    }
    Ok(())
}

fn render(template: &str, name: &str, sdk_path: &str) -> String {
    template
        .replace(APPLICATION_NAME_TOKEN, name)
        .replace(SDK_PATH_TOKEN, sdk_path)
}

fn usage() -> String {
    "usage:\n  dali app new <name> [--sdk-path <path>]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{create_project, render, validate_name};
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
            "name={{ application_name }} sdk={{ sdk_path }}",
            "demo",
            "../sdk",
        );
        assert_eq!(rendered, "name=demo sdk=../sdk");
    }

    #[test]
    fn creates_the_documented_project_files() -> Result<(), Box<dyn std::error::Error>> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = env::temp_dir().join(format!("dali-cli-app-new-{suffix}"));
        fs::create_dir(&root)?;
        create_project(&root, "demo", "../crates/dali-sdk")?;
        assert!(root.join("demo/Cargo.toml").is_file());
        assert!(root.join("demo/dali.toml").is_file());
        assert!(root.join("demo/src/main.rs").is_file());
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
