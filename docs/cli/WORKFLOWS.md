# Dali CLI Workflows

## Build and validate a cartridge

1. Build a linked native payload.
2. Run dali cartridge.
3. Run dali inspect.
4. Preserve the inspected cartridge as the deployment artifact.

The cartridge should not be copied to hardware before inspection succeeds.

## Repository demo workflow

~~~text
just cartridge-hello
dali inspect --input target/thumbv7em-none-eabihf/debug/hello.amrn
~~~

The generated cartridge can then be installed on the SD card using the
repository's documented hardware procedure.

## Hardware boundary

The CLI validates host-side cartridge data. It does not establish that a kernel
can initialize storage, load the cartridge, transfer control, emit logs, or
drive the application LED. Those claims require the procedures in
docs/mvp-acceptance/README.md.

## CI workflow

CI should run cartridge construction, cartridge inspection tests, formatting,
strict Clippy, and workspace checks. Hardware acceptance remains a separate
evidence record.
