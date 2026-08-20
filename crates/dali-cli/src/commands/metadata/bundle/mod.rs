//! Repository bundle generation, inspection, and host-side verification.

mod common;
mod generate;
mod verify;

const GENERATE_COMMAND: &str = "generate";
const INSPECT_COMMAND: &str = "inspect";
const VERIFY_COMMAND: &str = "verify";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(2).map(String::as_str) {
        Some(GENERATE_COMMAND) => generate::run(arguments),
        Some(INSPECT_COMMAND) => verify::inspect(arguments),
        Some(VERIFY_COMMAND) => verify::run(arguments),
        _ => Err("usage: dali metadata bundle {generate|inspect|verify} ...".to_owned()),
    }
}
