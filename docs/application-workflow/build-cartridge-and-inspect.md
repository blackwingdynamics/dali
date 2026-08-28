# Build a native payload

From the repository root, build the validation application for the embedded
target:

```text
cargo build -p dali-app-hello \
  --features embedded-payload \
  --target thumbv7em-none-eabihf
```

Convert the linked ELF into a raw binary payload:

```text
cargo objcopy -p dali-app-hello \
  --features embedded-payload \
  --target thumbv7em-none-eabihf \
  --bin dali-app-hello \
  -- -O binary target/thumbv7em-none-eabihf/debug/dali-app-hello.bin
```

The binary is the linked native image, not source code. The entry offset is
currently zero because the linker places the declared entry section at the
beginning of the payload.

The equivalent repository recipe is:

```text
just app-build
```

## Create an AMRN cartridge

The cartridge command adds the documented AMRN v1 header, records the payload
length and entry metadata, and calculates the CRC32 over the payload:

```text
cargo run -p dali-cli --bin dali -- cartridge \
  --input target/thumbv7em-none-eabihf/debug/dali-app-hello.bin \
  --output target/thumbv7em-none-eabihf/debug/hello.amrn \
  --entry-offset 0
```

The command rejects an empty payload, an oversized payload, an output that is
too small, or an invalid entry offset. It does not infer an entry symbol from
the ELF file; the caller supplies the byte offset explicitly.

The equivalent recipe, including the application build and binary extraction,
is:

```text
just cartridge-hello
```

The generated cartridge is written under the target directory and is ignored by
Git as a build artifact.

## Inspect an AMRN cartridge

Use the host CLI to validate an existing cartridge against the AMRN contract and
print its decoded fields:

```text
cargo run -p dali-cli --bin dali -- inspect \
  --input target/thumbv7em-none-eabihf/debug/hello.amrn
```

The command validates the header, target, ABI version, payload bounds,
execution entry, CRC32, and exact file length. It does not modify the cartridge.
