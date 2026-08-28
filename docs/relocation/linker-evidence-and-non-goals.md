# Initial linker evidence

The standalone `apps/dali-app-relocation-fixture` builds with the pinned
`thumbv7em-none-eabihf` toolchain and linker `--emit-relocs` option. Its final
ELF contains both `.rel.dali_code` and `.rel.dali_data` sections. The first
fixture produced these application relocation kinds:

- `R_ARM_THM_CALL`;
- `R_ARM_THM_MOVW_ABS_NC`;
- `R_ARM_THM_MOVT_ABS`;
- `R_ARM_ABS32`.

This proves that relocation records can be retained and that both code and
writable-data references are observable. The CLI extraction step recognizes
these records, but each kind still needs a bounded patch decoder, patch test,
and rejection test before it can be emitted to or applied from an AMRN package.

Build and inspect the fixture from its directory:

```text
cargo build --offline --features embedded-payload,abi-current \
  --target thumbv7em-none-eabihf
readelf -rW target/thumbv7em-none-eabihf/debug/dali-app-relocation-fixture
```

## Non-goals

This design does not add dynamic linking, shared libraries, arbitrary function
pointers into the kernel, application-owned interrupts, signatures, secure
boot, or multiple applications. Those require separate contracts and roadmap
tasks.
