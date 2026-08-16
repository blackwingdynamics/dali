# Dali Application Project Contract

This document defines the project structure that `dali app new` and
`dali app init` create.

## Commands

```text
dali app new <name> [--sdk-path <path>]
dali app init
```

`new` creates a new application directory and refuses to overwrite an
existing path. `init` initializes the current directory using its directory
name as the application name and refuses to replace any existing managed
file. `build` builds and packages locally; these commands do not flash
hardware or write to an SD card.

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
├── memory.v3.x
├── .cargo/
│   └── config.toml
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

The generated Cargo configuration adds the application linker script for the
documented embedded target. This keeps standalone projects outside the Dali
repository aligned with the same linker contract.

`memory.v3.x` is a target-manifest-generated, reviewable ABI v3 code/data
layout artifact. The current build pipeline does not select it; `memory.x`
remains the active ABI v2 linker script until the v3 SDK and artifact pipeline
are implemented.

`dali.toml` is the Dali project manifest. It owns application metadata and
build configuration that must remain configurable:

- application name and version;
- SDK requirement;
- target profile, expressed as the documented Rust compilation target;
- package output name;
- entry symbol and entry offset policy.

The generated project currently uses the `f405` target profile. The build
command passes this manifest-owned value to Cargo and does not embed a board
name or target mapping in command logic.

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
