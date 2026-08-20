#[path = "metadata_delegation.rs"]
mod delegation;

const DELEGATION_COMMAND: &str = "delegation";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(DELEGATION_COMMAND) => delegation::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: dali metadata delegation create ...".to_owned()
}
