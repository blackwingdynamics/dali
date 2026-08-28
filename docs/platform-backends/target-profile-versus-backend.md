# Target profile versus backend

`targets/*.toml` is the source of truth for declarative target facts: memory
ranges, clocks, pins, peripherals, package compatibility, and protection
capabilities. The generated target registry exposes typed metadata to host and
kernel build code.

The backend maps those facts to executable hardware operations. The
`kernel/src/platform/mod.rs` facade defines the crate-private `Backend` contract
and exposes only stable kernel-facing operations;
backend resource structs, PAC types, pin tuples, and clock objects stay behind
that facade. It must not
silently duplicate configurable values in Rust. A value that differs between
targets belongs in the manifest or in a documented processor-specific
implementation rule.
