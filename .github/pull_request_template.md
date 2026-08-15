## Summary

<!-- Describe the problem and the change. -->

## Scope

<!-- List the affected modules, packages, hardware, or documents. -->

## Validation

<!-- List exact commands and results. -->

```text
cargo fmt --all -- --check
cargo check --workspace --exclude dali-kernel
cargo check-kernel
cargo test -p dali -p dali-app-hello -p dali-cli
cargo clippy --workspace --all-targets --exclude dali-kernel -- -D warnings
cargo clippy -p dali-kernel --target thumbv7em-none-eabihf --bin dali-kernel -- -D warnings
git diff --check
```

## Hardware evidence

<!-- Include board, wiring, firmware, storage, expected output, and observed output. -->

Not applicable / attached:

## Limitations and follow-up work

<!-- Describe known limitations and linked roadmap tasks. -->

## Checklist

- [ ] I reviewed `CONTRIBUTING.md` and `docs/CODING_STANDARDS.md`.
- [ ] Code, comments, logs, and errors are in English.
- [ ] Every unsafe operation has an immediate `SAFETY` comment.
- [ ] External input and memory bounds are validated.
- [ ] Tests, documentation, or hardware evidence were updated.
- [ ] No unrelated changes are included.
- [ ] Commit messages follow Conventional Commits.
- [ ] No secrets or unreviewed generated artifacts are included.
