# Dali CLI Troubleshooting

## dali: command not found

Cargo's binary directory is not in PATH, or the CLI has not been installed.
Run the installation procedure and verify the configured Cargo binary path.

## cannot read input

Confirm that the path exists and points to the intended payload or package.
Use an absolute path while diagnosing path ambiguity.

## invalid AMRN cartridge

Run dali inspect on the exact artifact copied from the build output. Do not
rename a Rust source file or raw payload to .amrn; create the package with
dali package.

## CRC mismatch

Rebuild and package the payload again. Do not reuse a package after changing
the payload, entry metadata, or build profile.

## Hardware logs are missing

Successful CLI inspection proves only package validity. Check the separate
kernel flash, SD-card, USB CDC, and MVP acceptance procedures.

## Cargo package versus executable name

The Cargo package is dali-cli, while the installed executable is dali. Use
cargo run -p dali-cli --bin dali -- ... from a checkout, or dali ... after
installation.
