mod bundle;
mod delegation;
mod repository;

const DELEGATION_COMMAND: &str = "delegation";
const BUNDLE_COMMAND: &str = "bundle";
const REPOSITORY_COMMAND: &str = "repository";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(DELEGATION_COMMAND) => delegation::run(arguments),
        Some(BUNDLE_COMMAND) => bundle::run(arguments),
        Some(REPOSITORY_COMMAND) => repository::run(arguments),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: dali metadata {delegation|bundle|repository} ...".to_owned()
}
