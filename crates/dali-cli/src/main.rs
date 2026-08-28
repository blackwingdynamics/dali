//! Dali OS cartridge and device CLI.

use std::{env, process};

mod commands;

fn main() {
    if let Err(error) = commands::run(env::args().skip(1).collect()) {
        eprintln!("dali: {error}");
        process::exit(1);
    }
}
