# 5. Commenting standard

Comments must explain why code exists, which invariant it protects, or which hardware/protocol constraint applies. Comments must not merely repeat what the code already says.

Good:

```rust
// Wrapping arithmetic prevents heartbeat overflow from triggering a panic
// during long-running operation.
counter = counter.wrapping_add(1);
```

Bad:

```rust
// Increment the counter.
counter += 1;
```

Comment rules:

- Write comments as complete English sentences.
- Use punctuation consistently.
- Keep comments close to the code they explain.
- Explain safety assumptions immediately before the relevant operation.
- Explain protocol fields, hardware quirks, timing assumptions, and non-obvious invariants.
- Remove comments that become inaccurate after a code change.
- Never use comments to justify code that should instead be simplified.

The following are prohibited:

- commented-out code;
- decorative banner comments;
- stale or speculative comments;
- unexplained abbreviations;
- comments that contradict the implementation;
- untracked `TODO` items.

`TODO` is allowed only with a concrete issue or roadmap reference:

```rust
// TODO(#42): Replace polling with a DMA completion event.
```

## 6. Documentation comments

Every public type, function, constant, trait, and module must have a Rust documentation comment when its purpose or contract is not obvious from its name.

Documentation must describe observable behavior, not private implementation details. Use the appropriate sections:

- `# Errors` for fallible operations;
- `# Panics` for documented panic conditions;
- `# Safety` for unsafe functions or operations;
- hardware, timing, ownership, and blocking behavior when relevant.
