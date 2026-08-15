mod app;
mod inspect;
mod package;

const APP_COMMAND: &str = "app";
const PACKAGE_COMMAND: &str = "package";
const INSPECT_COMMAND: &str = "inspect";
const INPUT_FLAG: &str = "--input";
const OUTPUT_FLAG: &str = "--output";
const ENTRY_OFFSET_FLAG: &str = "--entry-offset";

pub(super) fn run(arguments: Vec<String>) -> Result<(), String> {
    let Some(command) = arguments.first() else {
        return Err(usage());
    };
    match command.as_str() {
        APP_COMMAND => app::run(&arguments),
        PACKAGE_COMMAND => package::run(&arguments),
        INSPECT_COMMAND => inspect::run(&arguments),
        _ => Err(usage()),
    }
}

pub(super) fn required_flag(arguments: &[String], flag: &str) -> Result<String, String> {
    let position = arguments
        .iter()
        .position(|argument| argument == flag)
        .ok_or_else(usage)?;
    arguments.get(position + 1).cloned().ok_or_else(usage)
}

fn usage() -> String {
    format!(
        "usage:\n  dali app new <name> [--sdk-path <path>]\n  dali app init [--sdk-path <path>]\n  dali app build\n  dali package {INPUT_FLAG} <payload> {OUTPUT_FLAG} <package> {ENTRY_OFFSET_FLAG} <bytes>\n  dali inspect {INPUT_FLAG} <package>"
    )
}
