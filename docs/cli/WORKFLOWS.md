# Dali CLI Workflows

## Build and validate a package

1. Build a linked native payload.
2. Run dali package.
3. Run dali inspect.
4. Preserve the inspected package as the deployment artifact.

The package should not be copied to hardware before inspection succeeds.

## Repository demo workflow

~~~text
just package-hello
dali inspect --input target/thumbv7em-none-eabihf/debug/hello.amrn
~~~

The generated package can then be installed on the SD card using the
repository's documented hardware procedure.

## Hardware boundary

The CLI validates host-side package data. It does not establish that a kernel
can initialize storage, load the package, transfer control, emit logs, or
drive the application LED. Those claims require the procedures in
docs/MVP_ACCEPTANCE.md.

## CI workflow

CI should run package construction, package inspection tests, formatting,
strict Clippy, and workspace checks. Hardware acceptance remains a separate
evidence record.
