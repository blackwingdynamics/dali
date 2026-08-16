# Install the Dali CLI

## Requirements

- Rust and Cargo from the repository toolchain;
- a checkout of Dali OS;
- the repository lockfile;
- Cargo's binary directory available in PATH for direct invocation.

## Install from the repository

Run from the repository root:

~~~text
cargo install --path crates/dali-cli --locked
~~~

The installed executable is dali. The Cargo package remains dali-cli.

## Verify installation

~~~text
dali inspect --input <package.amrn>
~~~

A valid package prints AMRN package valid and its decoded contract fields.

## Update or remove

Repeat the same cargo install command to replace the local executable with the
current checkout version. Remove the installed binary using the Cargo binary
directory configured on the host.

## Reproducibility

Use --locked so dependency resolution follows Cargo.lock. Do not install a
package from an unrelated checkout and assume it matches the kernel contract.
