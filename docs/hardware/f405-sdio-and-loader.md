# Current MVP SDIO board

- Board: WeAct Studio STM32F405RGT6 Core Board, 64-pin
- MCU: STM32F405RGT6
- HSE: 8 MHz
- Maximum documented MCU frequency: 168 MHz
- Status LED: PB2, active-high push-pull
- User key: PC13, active-low with pull-up
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
current AMRN cartridge target; AMRN target
compatibility remains defined by `amrn-format/README.md`.

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
