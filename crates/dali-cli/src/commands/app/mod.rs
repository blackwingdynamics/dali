mod artifacts;
mod build;
mod cartridge;
mod init;
mod new;
mod relocations;

const BUILD_COMMAND: &str = "build";
const INIT_COMMAND: &str = "init";
const CARTRIDGE_COMMAND: &str = "cartridge";
const NEW_COMMAND: &str = "new";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(NEW_COMMAND) => new::run(arguments),
        Some(INIT_COMMAND) => init::run(arguments),
        Some(BUILD_COMMAND) => build::run(arguments),
        Some(CARTRIDGE_COMMAND) => cartridge::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:\n  dali app new <name> [--sdk-path <path>]\n  dali app init [--sdk-path <path>]\n  dali app build\n  dali app cartridge"
        .to_owned()
}
