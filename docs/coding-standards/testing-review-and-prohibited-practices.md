# 13. Testing requirements

Every new module must have pure-logic tests, hardware evidence, or both.

Pure-logic tests should cover valid and invalid input, boundary values, truncation, integer overflow, CRC mismatch, unsupported versions, and unsupported targets.

The AMRN loader must test valid headers, invalid magic, invalid version, unsupported target, truncated input, oversized payload, out-of-range load address, invalid execution offset, CRC32 mismatch, and valid entry-point calculation.

Hardware-dependent code requires recorded evidence containing the board, wiring, firmware revision, storage media, power source, logging channel, and observed output.

Build success alone is not hardware acceptance evidence.

## 14. Code review checklist

Before accepting a change, review:

- Does it follow the architecture and roadmap?
- Does each module keep one responsibility?
- Is every unsafe operation necessary, minimal, and documented?
- Are all external inputs validated before use?
- Are bounds and overflow checks explicit?
- Are errors preserved and reported?
- Are magic numbers replaced with named constants or types?
- Do public APIs have accurate documentation?
- Do comments explain why rather than restating what?
- Is there a test or hardware evidence entry?
- Does the change avoid unrelated refactoring?
- Do formatting, linting, and diff validation pass?

## 15. Prohibited practices

The following practices are not accepted:

- undocumented `unsafe`;
- hardcoded hardware, configuration, or runtime values;
- rewriting large files for a small change;
- monolithic Rust files above the documented size limit;
- unreviewed `#[allow(...)]` attributes;
- undocumented API, ABI, AMRN, memory-layout, or security-contract changes;
- disabled, ignored, or removed tests without a tracked reason;
- silent fallback behavior for invalid external input;
- logging secrets or sensitive device data;
- unpinned or unexplained dependencies;
- destructive Git operations without explicit approval;
- claims of hardware completion without hardware evidence;
- runtime `unwrap()` or `expect()` without a reviewed invariant;
- magic memory addresses or pin numbers;
- unbounded allocation in a real-time path;
- silent error swallowing;
- commented-out code;
- stale comments;
- untracked TODO work;
- undocumented hardware assumptions;
- mixing unrelated responsibilities in one module;
- making security claims that are not backed by an implemented mechanism and evidence.
