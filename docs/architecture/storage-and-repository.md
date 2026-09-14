# 10. Storage subsystem

The storage subsystem is responsible for:

- SDIO initialization and SD-card communication;
- SD-card initialization at a safe low SPI frequency;
- block reads;
- FAT16/FAT32 read-only filesystem access;
- root-directory cartridge discovery;
- bounded reads into loader-owned buffers.

The boot watchdog is deliberately installed immediately before the selected
storage transport enters its opaque card-initialization routine. A HAL may own
an internal polling loop during that phase, so the board-agnostic storage layer
cannot safely inject a feed callback into it. The watchdog is therefore not
serviced inside that opaque phase; a non-responsive medium becomes bounded Safe
Mode recovery. Once the transport returns, the watchdog covers the first block
read, filesystem operation, repository stream, or
application handoff; those bounded kernel-owned paths service it through the
generic platform progress boundary.

The F405 SDIO block-read path keeps HAL card initialization but owns the data
FIFO drain in a board-local module. It uses the documented DMA2 Stream 3,
Channel 4 receive request and an aligned word buffer, then maps SDIO timeout,
CRC, overrun, and DMA errors to typed storage errors. MMIO access is
centralized there. The read command is issued without waiting for its response
so DMA can begin receiving as soon as the card asserts data activity. USB
servicing remains available while the storage transfer is blocking. SDIO
hardware flow control remains disabled because the STM32F405/F40x device
errata report clock glitches and CRC errors when it is enabled.
The DMA controller is reset before each transfer setup to remove stale stream
state, and a completed SDIO command with no active receive transfer is treated
as a bounded transport failure. DMA FIFO mode uses a full threshold and
incremental four-word bursts so the final receive words are not stranded below
the direct-mode request threshold. If the SDIO data counter reaches zero while
DMA has a bounded tail pending, that tail is drained directly from the FIFO.
The kernel exposes this transport through the `sdio` capability feature; board
features enable capabilities and select pins/clocks separately.

The hardware-neutral storage lifecycle tracks `Unavailable`, `Present`,
`Ready`, `Removed`, and `Fault` states. The F405 board has no card-detect GPIO,
so a bounded SDIO command or data timeout is classified as a removal
observation; integrity and transport errors remain faults. Initialization and
the first block read use a bounded reinitialization window before returning to
the kernel recovery heartbeat. When recovery retains the SDIO reader, the
heartbeat performs bounded presence probes and reinitializes the reader at the
configured recovery interval. A successful probe re-enters the normal
initialized-reader loading path, so a present cartridge can be verified and
launched without a kernel reset. If no card is present, the kernel remains in
its heartbeat and recovery state.

## Board-agnostic repository and durable-storage boundary

Repository loading and trust-store installation are defined above the board
storage implementation. The kernel exposes three logical boundaries:

```text
BlockDevice
    -> filesystem / durable-artifact adapter
    -> DurableStorageAdapter and RepositoryStreamStorage
    -> persistence coordinator and repository loader
```

`BlockDevice` owns fixed-block transfer only. `DurableStorageAdapter` owns the
kernel's bounded Slot A, Slot B, and commit-journal artifacts.
`RepositoryStreamStorage` owns logical metadata-document and content-addressed
AMRN streams. Neither
trait mentions SDIO, FAT, STM32, pins, clocks, DMA, or a physical filename.

The F405 implementation is one adapter chain below these contracts:

```text
STM32F405 SDIO -> F405 block adapter -> FAT32 adapter -> logical storage traits
```

The repository loader consumes `RepositoryStreamStorage` through bounded
two-pass role verification. The complete Root -> Timestamp -> Snapshot ->
Targets -> Delegation -> Revocation -> Cartridge -> AMRN chain is assembled before
the execution loader receives a cartridge. The loader does not enumerate
directories or choose cartridges from filenames. This keeps future board adapters
replaceable without changing trust policy or loader logic.

When the `repository-loader` feature is enabled, the F405 boot path constructs
the concrete FAT adapter, requests the board target profile, selects every
matching executable Binary v2 Targets record within the bounded execution
capacity, opens each lowercase content-addressed `amrns/<sha256>.amrn`
object, and passes the verified streams to the existing slot/relocation loader.
The target memory contract is resolved from each manifest-owned slot at the
dependency-injection boundary; the generic repository loader contains no F405
or SDIO types. The default MVP build keeps the legacy root-cartridge path because
repository boot remains feature-gated. The Binary v2 repository path has F405
development-profile hardware evidence, but it is not the default MVP profile.

The concrete F405 adapter is `FatRepositoryStorage<D>`. Its constructor accepts
an explicit `RepositoryMetadataFormat` (`JsonV1` or `BinaryV2`) and resolves the
board-agnostic logical documents through `metadata/`,
`metadata/delegat/`, and `amrns/`, using FAT-compatible bounded directory names, while durable artifacts remain
the kernel-owned root files `DALI-ACT.BIN`, `DALI-CAN.BIN`, and `DALI-CMT.BIN`.
The adapter also exposes `with_content_addressed_cartridge()`, which opens only
the lowercase SHA-256 cartridge filename under `amrns/` and hands the file to
the existing bounded AMRN execution loaders.

## Additional artifact sources

SDIO is one storage adapter, not the kernel's permanent cartridge source. A
future flash-backed artifact store, NVMe adapter, or other durable medium must
implement the hardware-neutral `ArtifactSource` contract in
`kernel/src/storage/repository.rs`. USB CDC is an installation transport only:
it may deliver an AMRN artifact to a staged persistent-storage region, but it
must not become a second loader policy or be mixed with the logging stream. The
accepted architecture and staged rollout are recorded in
[ADR-0001](adr/0001-multi-source-artifact-storage.md).

The public backend API also defines a bounded `ArtifactReader` contract for
read-only byte-range access. It describes the transport boundary only; AMRN
validation and execution remain owned by the kernel loader.

The adapter now exposes the streaming contract directly. The shared Binary v2
envelope parser validates fragmented envelopes without retaining their body;
typed Root, Timestamp, Snapshot, Delegation, and Revocation parsers are
organized under `dali-metadata/src/parser/streaming/`, and
`verify_binary_role_envelope` provides the parse-and-replay second pass. The
kernel repository loader now also contains a bounded two-pass AMRN v5 validator
under `loader/repository/amrn.rs`, a generic role-stream capture/replay helper
under `loader/repository/chain.rs`, Root anchor membership wiring through the
target manifest, and the board-agnostic `load_binary_repository()` chain
assembler. `load_repository_cartridge()` is the boot handoff: it injects the F405
adapter into the generic chain and then opens only the digest-selected cartridge
for the existing execution pipeline.

## Production multi-application policy

The legacy root-file path intentionally accepts exactly one root `.amrn`
cartridge and rejects ambiguous selection. The feature-gated Binary v2
repository path already selects multiple executable Targets records within a
bounded capacity and maps each verified cartridge to a manifest-owned slot.
The feature-gated runtime can execute declared contexts through its
hardware-verified scheduler and MPU switching path, but it is not yet a
general production multi-application policy. Dali must still define lifecycle,
replacement, restart, recovery, and application-to-application behavior for
missing, duplicate, incompatible, or already-reserved cartridges. The slot
manager must consume validated selection; it must not infer ownership from
directory order or cartridge names.
