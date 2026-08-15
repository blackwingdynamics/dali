mod app;
mod doctor;
mod inspect;
mod package;
mod target;
mod target_info;

const APP_COMMAND: &str = "app";
const DOCTOR_COMMAND: &str = "doctor";
const PACKAGE_COMMAND: &str = "package";
const INSPECT_COMMAND: &str = "inspect";
const TARGET_COMMAND: &str = "target";
const INPUT_FLAG: &str = "--input";
const OUTPUT_FLAG: &str = "--output";
const ENTRY_OFFSET_FLAG: &str = "--entry-offset";

pub(super) fn run(arguments: Vec<String>) -> Result<(), String> {
    let Some(command) = arguments.first() else {
        return Err(usage());
    };
    match command.as_str() {
        APP_COMMAND => app::run(&arguments),
        DOCTOR_COMMAND => doctor::run(&arguments),
        PACKAGE_COMMAND => package::run(&arguments),
        INSPECT_COMMAND => inspect::run(&arguments),
        TARGET_COMMAND => target::run(&arguments),
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
        "usage:\n  dali doctor\n  dali target list\n  dali target info <profile> [--field probe-chip]\n  dali target scaffold <profile> [--output <workspace-root>]\n  dali app new <name> [--sdk-path <path>]\n  dali app init [--sdk-path <path>]\n  dali app build\n  dali package {INPUT_FLAG} <payload> {OUTPUT_FLAG} <package> {ENTRY_OFFSET_FLAG} <bytes>\n  dali inspect {INPUT_FLAG} <package>"
    )
}
