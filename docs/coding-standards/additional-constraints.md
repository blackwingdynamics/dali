# 4. Additional strict constraints

## Unsafe boundaries

`unsafe` must remain centralized in `board`, `loader/exec`, or an explicitly documented low-level module. It must not spread into parsers, storage policy, SDK APIs, or ordinary runtime logic.

### Lint suppression

Do not use `#[allow(...)]` to hide warnings, safety issues, dead code, or failed lint rules. An exception must identify the exact warning, explain why it is safe, and be reviewed.

### Contract changes

Changing an ABI, AMRN field, memory layout, public API, or security claim requires an update to the relevant specification, tests, and versioning documentation in the same change.

### Dependencies

New dependencies require review of `no_std` compatibility, license, maintenance, auditability, transitive dependencies, binary size, RAM cost, and security history. Important tools and dependencies must be pinned where reproducibility matters.

### Completion claims

Compilation is not completion. Reports must distinguish source compilation, host tests, embedded target checks, hardware tests, and MVP acceptance. Do not claim hardware or acceptance completion without the required evidence.

### Tests

Do not disable, ignore, or remove tests without an issue or roadmap reference and a documented reason. “Test later” is not acceptable evidence of completion.

### Fallback behavior

Invalid configuration, missing storage, malformed packages, and hardware failures must not trigger silent fallback behavior unless that fallback is explicitly documented and safe.

### Hardware evidence boundary

Do not add fake hardware, fake devices, fake storage, fake readers, or
simulated runtime behavior as a substitute for the real implementation or
hardware acceptance. Host tests may cover genuinely hardware-neutral codecs,
validators, and pure contracts over caller-owned bytes, but those tests are not
hardware evidence. A test application is valid only when it is a real compiled
package executed through the documented target contract.

### Logging and secrets

Never log credentials, keys, private data, sensitive device information, or unnecessary memory dumps.

### Change scope

Mass formatting, renaming, migration, and repository-wide rewrites are forbidden outside the explicit task scope. Preserve unrelated user changes.

### Git safety

Destructive operations such as `git reset --hard`, `git clean -fd`, force push, and bulk deletion require explicit approval.

### Function complexity

Functions should normally remain below 50 lines, use no more than three nesting levels, and perform one responsibility. Split a function when it exceeds these limits unless the exception is documented in review.

### Reproducibility

Preserve lockfiles, pin important tools and dependencies, and document commands that produce generated artifacts. A local success that cannot be reproduced in CI is not an accepted validation result.
