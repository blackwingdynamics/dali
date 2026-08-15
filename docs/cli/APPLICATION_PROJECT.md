# Dali Application Project Contract

This document defines the project structure that `dali app new` and
`dali app init` will create. The initial `new` command is implemented; `init`
remains planned.

## Commands

```text
dali app new <name> [--sdk-path <path>]
dali app init
```

`new` creates a new application directory and refuses to overwrite an
existing path. `init` initializes the current directory and refuses to replace
existing managed files. Neither command builds, packages, flashes, or writes
to an SD card.

The command works both inside and outside a Dali workspace. Inside a workspace
it discovers the local SDK automatically. Outside a workspace, the caller must
provide `--sdk-path` pointing to a local checkout of the `dali` SDK crate. The
CLI stores a relative SDK path in the generated Cargo manifest and never writes
an absolute machine path into the project.

## Generated project

The scaffold must contain:

```text
<application>/
├── Cargo.toml
├── dali.toml
├── build.rs
├── memory.x
└── src/
    ├── lib.rs
    └── main.rs
```

The generated `Cargo.toml` is a native Rust application manifest. The
application SDK is referenced through the project configuration rather than
through a board-specific implementation path embedded in the command.
The manifest declares an independent Cargo workspace so a scaffold created
inside the Dali repository does not silently modify the root workspace's
member set.

`dali.toml` is the Dali project manifest. It owns application metadata and
build configuration that must remain configurable:

- application name and version;
- SDK requirement;
- target profile;
- package output name;
- entry symbol and entry offset policy.

Board pins, memory addresses, ABI versions, and AMRN protocol values remain
owned by documented target profiles and the AMRN/ABI contracts. The scaffold
command must not duplicate those values in command logic.

## Safety and overwrite policy

- An existing non-empty target directory is rejected.
- Existing managed files are never silently overwritten.
- The command creates parent directories only inside the requested project
  path.
- Invalid application names fail before any filesystem mutation.
- An external project without `--sdk-path` fails before any filesystem mutation.
- The SDK path must identify a Cargo crate named `dali`.
- A failed scaffold must report the path and operation that failed.

## Implementation order

1. Add host-side manifest and name validation tests. (Complete.)
2. Implement `dali app new <name> [--sdk-path <path>]`. (Complete.)
3. Implement `dali app init` using the same renderer and overwrite policy.
4. Add command documentation and fixture comparison tests. (Complete for `new`.)
5. Connect the manifest to `dali app build` and `dali app package`.
