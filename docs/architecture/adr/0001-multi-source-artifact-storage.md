# ADR-0001: Multi-source artifact storage

Status: **Accepted for implementation on `kernel-foundation`**

## Context

Dali currently boots cartridges through the F405 SDIO and FAT/repository
adapter. Reflashing the complete firmware or physically moving an SD card for
every AMRN test slows development and increases hardware handling. Future
targets may provide internal or external flash, USB mass storage, NVMe, or
another durable medium.

The kernel must not make SDIO the permanent definition of storage. USB is an
installation transport, not the persistent artifact medium and not a loader
policy.

## Decision

Define three independent boundaries:

1. `ArtifactSource` — supplies validated logical AMRN/repository streams to the
   loader;
2. `PersistentStorage` — exposes bounded medium operations and capabilities
   such as read, write, erase, flush, and atomic commit;
3. `InstallTransport` — receives a bounded artifact from USB, SWD, or another
   host transport and commits it through the persistent-storage contract.

The existing SD/FAT/repository chain remains one adapter implementation. A
flash-backed artifact store will be a second adapter. Future NVMe or other
media must implement the same logical contract without changing loader or AMRN
policy code.

The first implementation milestone is read-only boot from a target-owned flash
artifact region. USB installation is a subsequent milestone and must use a
staging region, complete AMRN validation, integrity/authentication checks, and
an atomic metadata commit before an artifact becomes bootable.

## Constraints

- Generic kernel policy must not contain SD, flash, NVMe, USB, vendor, or board
  identifiers.
- Flash regions, erase units, write alignment, and capacity belong to target
  metadata and the selected backend.
- The firmware image region and artifact region must be disjoint and validated
  by the backend/linker contract.
- A partial USB transfer or power loss must never make an uncommitted artifact
  bootable.
- Repeated test installation must not silently claim unlimited flash endurance;
  wear and erase policy must be explicit.
- USB CDC logging and artifact installation must have separate bounded state and
  ownership. Logging traffic must not be interpreted as installer input.
- Host tests may cover codecs, metadata, and state machines, but target flash
  persistence and power-loss behavior require hardware evidence.

## Consequences

The loader and AMRN validation code become reusable across SD, flash, and
future storage adapters. The board backend owns unsafe flash operations and
the target memory map. Development gains a USB-to-flash workflow without
changing the production SD path. The additional metadata, staging, and
recovery work is required before declaring flash installation reliable.
