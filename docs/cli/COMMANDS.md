# Dali CLI Commands

## Command syntax

The executable is dali. When running from the repository without installing
it, use Cargo's package and binary selectors:

~~~text
cargo run -p dali-cli --bin dali -- <command> <options>
~~~

## Available commands

### dali package

Creates an AMRN package from a linked native payload.

See commands/package.md.

### dali inspect

Validates an AMRN package and prints its decoded fields without modifying the
input.

See commands/inspect.md.

## Unsupported commands

Unknown commands fail with usage information. Hardware flashing, SD-card
installation, device discovery, package listing, and interactive terminal
operations are not currently CLI commands.
