# 10. Module and API boundaries

Each module must have one clear responsibility:

- `board`: reference-board pins, clocks, and peripheral ownership;
- `storage`: SD and filesystem access;
- `loader`: AMRN parsing, CRC32 validation, bounds checking, and execution;
- `runtime`: future tasks, scheduling, IPC, services, and watchdog policy;
- `main.rs`: bootstrap orchestration only.

Additional rules:

- Keep implementation details private.
- Use `pub` only for an intentional module contract.
- Do not use wildcard imports.
- Do not create circular module dependencies.
- The parser must not depend on hardware.
- The loader must not depend on filesystem internals.
- Hardware-specific code must not leak into cartridge-format logic.
- Native application execution must remain separate from cartridge parsing.

## 11. Naming and data modeling

- Types and traits use `PascalCase`.
- Functions, modules, and variables use `snake_case`.
- Constants use `SCREAMING_SNAKE_CASE`.
- Boolean names use `is_`, `has_`, `can_`, or `should_` where appropriate.
- Prefer `Sram`, `Crc32`, and `Spi` in type names over all-capital abbreviations.
- Represent protocol states and target identifiers with enums or named constants.
- Use newtypes for values whose units or meaning must not be confused.
- Do not pass raw `u32` values across an API when a named type can express the contract.

## 12. Logging standard

Logs must be written in English and remain deterministic enough for hardware diagnosis.

Use stable subsystem prefixes:

```text
[INFO][BOOT] System clock: 100 MHz
[INFO][SD] Card initialized
[INFO][AMRN] AMRN cartridge discovered in the card root
[INFO][AMRN] CRC32 valid
[INFO][AMRN] Loading payload: 2048 bytes at 0x20008000
[INFO][AMRN] Jumping to entry point
```

Logging rules:

- use stable subsystem prefixes;
- use the logging facade rather than calling a transport backend directly;
- keep color output optional through the `log-colors` feature;
- log validation failures with a reason;
- do not hide storage or loader errors behind a generic message;
- do not log credentials, keys, or secrets;
- avoid high-frequency logs in production paths;
- keep the MVP application proof on the LED and verify the bounded logging ABI independently.
