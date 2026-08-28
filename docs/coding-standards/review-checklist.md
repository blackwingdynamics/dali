# Engineering Review Checklist

Use this checklist for code, hardware, documentation, and generated-artifact
changes. It complements the repository contribution rules and does not replace
the owning technical contract.

## Scope and architecture

- [ ] The change maps to a roadmap task and has one clear objective.
- [ ] The owning module and affected boundaries are identified.
- [ ] No boot path, ABI, memory layout, cartridge layout, or feature contract is
      changed without the required architecture approval and documentation.
- [ ] Frozen subsystem paths remain unchanged unless an explicit exception was
      approved.

## Code and unsafe operations

- [ ] Hardware-specific values come from named constants, typed configuration,
      target manifests, or generated profiles.
- [ ] Hardware-neutral crates contain no PAC, HAL, register, pin, or board
      assumptions.
- [ ] Unsafe code is minimal, centralized, and has an immediate safety
      comment stating its invariant.
- [ ] Errors are typed, results are handled, and realtime paths are bounded.
- [ ] Tests cover the changed failure cases at the appropriate validation
      layer.

## Hardware and evidence

- [ ] The board, MCU, wiring, firmware revision, power source, and transport
      are recorded for physical claims.
- [ ] Host tests, embedded checks, flashing, debugger inspection, and Silicon
      Trace are identified separately.
- [ ] No hardware result is inferred from compilation, flashing, enumeration,
      or host tests.
- [ ] Incomplete hardware validation is marked unverified and its limitation
      is recorded.

## Documentation claims

- [ ] Public behavior, contract changes, limitations, and security claims are
      reflected in the owning documentation.
- [ ] Links, headings, terminology, commands, and status vocabulary pass the
      documentation validator.
- [ ] Examples are reproducible and do not contain credentials or invented
      hardware behavior.

## Generated artifacts and handoff

- [ ] Generated files are produced by the documented tool rather than edited
      manually.
- [ ] Generated changelogs and local build artifacts are excluded unless the
      repository explicitly requires them.
- [ ] The staged file list contains only the intended atomic change.
- [ ] Validation output and remaining limitations are included in the handoff.
