# Dali CLI Testing

## Unit tests

Command modules should test validation and formatting without hardware or
external devices. Current inspection tests cover valid packages and trailing
data rejection.

The `dali-device` crate tests discovery-record ordering, same-transport
deduplication, and stable transport/state/capability spellings without host
hardware.

The CLI also tests the pure Linux udev-property parser used by CDC discovery.
The adapter can be exercised on Linux with `dali device list`; a visible
`/dev/ttyACM*` device is required for a live CDC record. A CDC record without
a serial number must remain `unidentified`.

## Integration checks

The host validation set includes:

~~~text
cargo test -p dali -p dali-app-hello -p dali-cli
cargo clippy --workspace --all-targets --exclude dali-kernel -- -D warnings
cargo fmt --all -- --check
~~~

## Artifact checks

The package workflow should build an application, create an AMRN artifact, and
inspect that artifact with the installed dali executable.

## Hardware boundary

CLI tests do not prove SDIO, USB CDC, loader execution, or application LED
behavior. Those require target checks and documented physical evidence.
