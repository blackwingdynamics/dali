mod bundle;
mod delegation;

const DELEGATION_COMMAND: &str = "delegation";
const BUNDLE_COMMAND: &str = "bundle";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(DELEGATION_COMMAND) => delegation::run(arguments),
        Some(BUNDLE_COMMAND) => bundle::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: dali metadata {delegation|bundle} ...".to_owned()
}
