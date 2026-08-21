# Reference Hardware

## Board

The project currently has one compile-time kernel backend. The F405 board is
the MVP execution platform. A BlackPill profile is retained only as a
generator input and has no kernel backend.

### Generator-only board profile

- Board: WeAct BlackPill
- MCU: STM32F411CEU6
- Core: ARM Cortex-M4F
- Frequency target: 100 MHz
- Application target: not supported by the current kernel

## SRAM layout

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

The application load address is fixed at `0x20008000` for the MVP.

## Current MVP SDIO board

- Board: WeAct Studio STM32F405RGT6 Core Board, 64-pin
- MCU: STM32F405RGT6
- HSE: 8 MHz
- Maximum documented MCU frequency: 168 MHz
- Status LED: PB2, active-high push-pull
- User key: PC13
- Programming: USB DFU or SWD on PA13/PA14
- SD interface: hardware SDIO, 4-bit mode

| Function | Pin |
| --- | --- |
| SDIO clock | PC12 |
| SDIO command | PD2 |
| SDIO data 0 | PC8 |
| SDIO data 1 | PC9 |
| SDIO data 2 | PC10 |
| SDIO data 3 | PC11 |

The F405 board is selected with the `board-stm32f405-sd` Cargo feature. Its
SDIO pin tuple is owned by the board backend and is consumed by the storage
subsystem. The current F405 backend uses HAL card initialization and a
board-local DMA2 Stream 3, Channel 4 receive path with an aligned word buffer
for block reads. Before programming DMA2, the kernel validates that buffer
against the target manifest's DMA-visible region and retains exclusive mutable
ownership for the transfer. This protects the current kernel SDIO path; it is
not a claim of general application or peripheral DMA isolation. It is the
current AMRN package target; AMRN target
compatibility remains defined by `AMRN_FORMAT.md`.

The manufacturer and target metadata for this board is declared in
`targets/f405.toml`. The F405 backend consumes the generated clock profile and
performs compile-time checks against the AMRN load-region and ABI contracts.
Typed GPIO and peripheral ownership remains explicit in the backend because
the HAL requires compile-time pin types and singleton peripheral ownership.

The STM32F405 also provides CCM RAM at `0x10000000`. Its suitability for the
privileged kernel runtime and stack is a future isolation design question;
peripheral DMA buffers must remain in DMA-accessible SRAM.

## Current repository-loader evidence

On 2026-08-21, the feature-gated `repository-loader` F405 release profile was
flashed with a signed Binary v2 bundle prepared by
`scripts/prepare-f405-binary-v2-sd.sh`. The board initialized SDIO, read the
repository, verified the AMRN header, payload, and signature, loaded one
manifest-declared package into slot 1, and executed the relocation fixture.
This is development-profile hardware evidence for the configured release
anchor. It does not establish production key custody, Secure Boot, DMA
isolation, revocation rejection on target, or interrupted-write recovery.

## Multi-slot isolation layout

The following layout is active in the F405 target manifest and is generated
from target metadata rather than embedded in kernel policy:

```text
0x20000000 - 0x20007FFF   Kernel DMA and transport buffers, 32 KiB
0x20008000 - 0x2000BFFF   Application slot 0 code, 16 KiB
0x2000C000 - 0x2000FFFF   Application slot 0 data/PSP, 16 KiB
0x20010000 - 0x20013FFF   Application slot 1 code, 16 KiB
0x20014000 - 0x20017FFF   Application slot 1 data/PSP, 16 KiB
0x20018000 - 0x2001FFFF   Runtime-region repository workspace, 32 KiB
0x10000000 - 0x1000FFFF   Kernel runtime, static state, and privileged stack
```

The kernel runtime migration must prove that every DMA-visible buffer remains
in the first SRAM region; CCM is not DMA-accessible on this MCU. The repository
workspace is a bounded, privileged BSS allocation selected from the target
manifest's runtime region. It is not application memory and is not implicitly
shared with unprivileged applications. Any future shared-memory policy must
assign explicit MPU permissions and ownership.

## Electrical requirements

The SD interface must use the board's correct 3.3 V logic levels. The SD module, wiring, power supply, and chip-select pull-up behavior must be verified on the actual hardware before acceptance testing.

## Clock and logging

The reference kernel targets the F405 168 MHz system clock from its 8 MHz HSE
and supports both RTT and USB CDC logging. The backend configures the USB FS
48 MHz clock domain and uses PA11/PA12 for USB D-/D+.

## Hardware acceptance evidence

Each hardware milestone should record board revision, wiring, firmware revision, SD-card type/filesystem, power source, logging channel, and observed output.
