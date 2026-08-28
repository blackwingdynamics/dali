# Signed package acceptance

## Repository loader hardware acceptance status (2026-08-22)

Repository streaming watchdog progress is injected through the target
platform boundary. The FAT repository adapter invokes the bounded progress
hook only after a non-empty chunk has been accepted by the parser, hash, or
signature consumer. Storage or verification failures therefore do not feed
the watchdog. The repository loader remains board-agnostic; the platform
supplies the hook that maps valid chunk progress to the hardware watchdog
service operation.

The cryptographic replay path additionally divides each delivered body chunk
into bounded 64-byte verification units and invokes the same progress hook
after each unit. This prevents a single Ed25519/SHA-256 update from exceeding
the declared watchdog window while preserving the rule that only successfully
processed bytes can advance the watchdog.

### Binary v2 host and linker evidence

The Binary v2 CLI dispatch and metadata codec were checked with:

```text
cargo test -p dali-cli -p dali-metadata
51 dali-cli tests passed
54 dali-metadata tests passed
cargo check -p dali-kernel --no-default-features --features board-stm32f405-sd,usb-cdc,abi-relocation,repository-loader,storage-write --target thumbv7em-none-eabihf
passed
```

The release image was rebuilt with the repository-loader feature set,
`CARGO_PROFILE_RELEASE_DEBUG=2`, and a linker map at
`/tmp/dali-f405-memory.map`. The exact build configuration was:

```text
CARGO_PROFILE_RELEASE_DEBUG=2 cargo build -p dali-kernel --release --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write \
  --target thumbv7em-none-eabihf
```

The generated linker map, target LLVM size report, and symbols from the
2026-08-22 report show:

```text
.dma_buffer           0x1200 =  4608 bytes @ 0x20000000
.fault_capture          0x44 =    68 bytes @ 0x10000000
.repository_workspace 0x7640 = 30272 bytes @ 0x20018000
.text                0x37c84 = 228484 bytes @ 0x080001a8
.rodata                0x7004 =  28676 bytes @ 0x08037e30
.data                 0x080c =  2060 bytes @ 0x10000080
.bss                  0x1748 =  5960 bytes @ 0x1000088c
_stack_end                         0x10001fd4
_stack_start                       0x10010000
available CCM stack               0x0000e02c = 57388 bytes
runtime region remaining          0x000009c0 =  2496 bytes
```

The repository workspace is now a target-profile-owned, board-agnostic BSS
section in the declared runtime region. It contains the shared stream chunk,
AMRN pass state, and typed metadata outputs; it is not placed in CCM, so it
does not consume the privileged kernel stack budget. The previous repository
loader frame overflow is no longer present in this build. Static prologue
inspection of the release ELF measured these largest repository-path local
allocations:

```text
load_repository_package       0x251c = 9500 bytes
replay_targets                 0xdfc = 3580 bytes
capture_role_into              0xce4 = 3300 bytes
parse_targets_first_pass       0xcdc = 3292 bytes
verify_packages_into            0xc74 = 3188 bytes
load_file_with_key              0xa3c = 2620 bytes
verify_timestamp_and_snapshot  0x83c = 2108 bytes
```

These are static function-prologue measurements, not a proof of the maximum
whole-program call depth. The 32 KiB runtime-region headroom is also distinct
from the CCM stack headroom: the former is workspace capacity, while the
latter is the kernel's privileged stack space.

`cargo-bloat` is not installed in the validation environment. The measurements
above were obtained with the target LLVM `llvm-size`, `llvm-nm`, and
`llvm-objdump`; no network installation was attempted. The map is reproducible
with the build command above and is not a firmware artifact. This evidence
does not claim F405 hardware acceptance by itself.

The production CI memory artifact is generated reproducibly with:

```text
DALI_REPORT_DIR=/tmp/dali-f405-memory-report scripts/report-f405-memory.sh
```

It contains the production ELF hash, target/features metadata, LLVM section
sizes and symbols, section headers, and the linker map. The artifact is CI
output and is not committed to the repository.

The feature-gated board-agnostic repository loader compiles for
`thumbv7em-none-eabihf`, and its host-visible boundary tests pass. A real F405
acceptance run on 2026-08-21 used the release-anchor bundle prepared by
`scripts/prepare-f405-binary-v2-sd.sh` and produced:

```text
[STORAGE] SDIO card initialized
[STORAGE] Trust-store artifact write/read-back test passed
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
[SECURITY] AMRN signature verified
[LOADER] Loaded 1 application package(s) into declared slots
[LOADER] Slot 1 (slot1) boundaries: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[APP] Relocation fixture
```

This is hardware evidence for the configured F405 release-anchor path:
SDIO access, Binary v2 repository traversal, AMRN validation, signature
verification, slot loading, and relocation-fixture execution. It does not
prove production key custody, pre-reset Secure Boot, arbitrary DMA isolation,
or power-loss recovery. The remaining repository acceptance scenarios are candidate
write/read-back with atomic activation, revoked-developer-key rejection on
target, and interrupted-write/power-loss recovery.

#### Final F405 Binary v2 acceptance record — 2026-08-22

The kernel was rebuilt with the release repository-loader feature set and
flashed to the WeAct Studio STM32F405RGT6 through the Raspberry Pi Pico 2
CMSIS-DAP probe. The SD card contained the freshly generated release-anchor
Binary v2 bundle. The package digest was:

```text
53bf7b2bd3af6b230802fc71f5ca006b67a980e3e4587c0973be140386af3d04
```

The observed F405 output at `2026-08-22 12:27:00` was:

```text
[STORAGE] SDIO card initialized
[STORAGE] Trust-store artifact write/read-back test passed
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
[SECURITY] AMRN signature verified
[LOADER] Loaded 1 application package(s) into declared slots
[LOADER] Slot 1 (slot1) boundaries: code=0x20010000+16384 data=0x20014000+16384 psp_top=0x20015010
[SECURITY] Application lifecycle: Ready
[SECURITY] Active application context: Running
[APP] Relocation fixture
```

This closes the normal Binary v2 F405 acceptance path for the recorded build:
SDIO initialization, trust-store access, repository traversal, signed AMRN
verification, slot loading, relocation, and `Ready -> Running` application
execution. The package file size, board revision, and power source were not
captured in this run and remain unspecified evidence fields. The separate
watchdog reset/Safe Mode scenario remains a distinct test.

```text
Board revision: Not recorded
Wiring: Exact SWD, USB CDC, and SD-card wiring not recorded
Firmware revision: Not recorded separately from the 2026-08-22 build record
Power source: Not recorded
Transport: USB CDC console; Pico 2 CMSIS-DAP for flashing
Expected trace: SDIO, trust-store, signed AMRN validation, slot loading, and Running state
Observed trace: All Binary v2 trace lines listed above
Limitations: Package size and physical power details were not captured; this is not Secure Boot or power-loss evidence
```

- [x] Host AMRN tests cover v5 header/trailer split parsing and signed-range
  boundary validation.
- [x] The feature-gated kernel target build covers v5 streaming signature,
  CRC32, relocation, and target-profile trust-anchor checks before SRAM copy.
- [x] The loader service-capability policy accepts the declared Log service and
  rejects undeclared required-service bits in host/unit coverage.
- [x] A real v5 relocation package is generated and inspected with the
  development test key; this is host/package evidence, not hardware evidence.
- The F405 signed-loader hardware image must be built with Cargo's `release`
  profile; the feature-complete development link does not fit the board's
  documented flash region.
- [x] F405 hardware: provision the documented development test public key and
  execute a valid signed v5 package; the kernel verified the signature before
  loading slot 1 and the relocation fixture ran.
- [x] F405 hardware: provision the generated release public trust anchor and
  execute a release-profile signed v5 package; the kernel logged
  `AMRN signature verified`, loaded one package into slot 1, and ran the
  relocation fixture. This verifies the configured release trust-anchor path
  on the development board; production key custody, rotation, and Secure Boot
  remain separate acceptance requirements.
- [x] F405 hardware: reject an unknown key ID before SRAM copy; the loader
  returned `UnknownTrustAnchor` and entered the kernel heartbeat.
- [x] F405 hardware: reject a modified signed payload; the loader returned
  `V5SignedPackage(CrcMismatch)` and entered the kernel heartbeat.
- [x] F405 hardware: reject a truncated DSIG trailer; the loader returned
  `V5SignedPackage(InvalidSignature)` and entered the kernel heartbeat.
- [ ] Secure Boot and kernel-image authenticity.
- [x] F405 hardware: run the feature-gated trust-store artifact
  write/flush/read-back acceptance path on a disposable FAT32 card and record
  the console result. Candidate durability is verified before commit-marker
  durability; power-loss recovery remains separate.
