#[path = "app_artifacts.rs"]
mod artifacts;
#[path = "build.rs"]
mod build;
#[path = "init.rs"]
mod init;
#[path = "new.rs"]
mod new;
#[path = "app_package.rs"]
mod package;

const BUILD_COMMAND: &str = "build";
const INIT_COMMAND: &str = "init";
const PACKAGE_COMMAND: &str = "package";
const NEW_COMMAND: &str = "new";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(NEW_COMMAND) => new::run(arguments),
        Some(INIT_COMMAND) => init::run(arguments),
        Some(BUILD_COMMAND) => build::run(arguments),
        Some(PACKAGE_COMMAND) => package::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:\n  dali app new <name> [--sdk-path <path>]\n  dali app init [--sdk-path <path>]\n  dali app build\n  dali app package"
        .to_owned()
}
