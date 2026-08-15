#[path = "init.rs"]
mod init;
#[path = "new.rs"]
mod new;

const INIT_COMMAND: &str = "init";
const NEW_COMMAND: &str = "new";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(NEW_COMMAND) => new::run(arguments),
        Some(INIT_COMMAND) => init::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:\n  dali app new <name> [--sdk-path <path>]\n  dali app init [--sdk-path <path>]"
        .to_owned()
}
