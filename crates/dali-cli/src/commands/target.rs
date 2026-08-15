const LIST_COMMAND: &str = "list";

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    match arguments.get(1).map(String::as_str) {
        Some(LIST_COMMAND) if arguments.len() == 2 => print_targets(),
        _ => Err(usage()),
    }
}

fn print_targets() -> Result<(), String> {
    println!("Supported Dali targets:");
    for target in dali_targets::SUPPORTED_TARGETS {
        println!("- {}", target.name);
        println!("  board: {}", target.board);
        println!("  mcu: {}", target.mcu);
        println!("  rust_target: {}", target.rust_target);
        println!("  amrn_target_id: 0x{:02X}", target.amrn_target_id);
        println!("  abi_version: {}", target.abi_version);
    }
    Ok(())
}

fn usage() -> String {
    "usage:\n  dali target list".to_owned()
}

#[cfg(test)]
mod tests {
    use dali_targets::SUPPORTED_TARGETS;

    #[test]
    fn exposes_the_current_application_target() {
        assert_eq!(SUPPORTED_TARGETS.len(), 1);
        assert_eq!(SUPPORTED_TARGETS[0].name, "f405");
        assert_eq!(SUPPORTED_TARGETS[0].amrn_target_id, 0x02);
    }
}
