# Reference Hardware

## Board

The project supports two compile-time board backends. The F405 board is the
current MVP execution platform; the BlackPill remains a separate backend.

### Secondary board

- Board: WeAct BlackPill
- MCU: STM32F411CEU6
- Core: ARM Cortex-M4F
- Frequency target: 100 MHz
- Application target: `thumbv7em-none-eabihf`

## SRAM layout

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

The application load address is fixed at `0x20008000` for the MVP.

## MVP pins

| Function | Pin |
| --- | --- |
| Status LED | PC13 |
| SPI1 SCK | PA5 |
| SPI1 MISO | PA6 |
| SPI1 MOSI | PA7 |
| SD chip select | PA4 |

### Current MVP SDIO board

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
for block reads. It is the current AMRN package target; AMRN target
compatibility remains defined by `AMRN_FORMAT.md`.

The manufacturer and target metadata for this board is declared in
`targets/f405.toml`. The F405 backend consumes the generated clock profile and
performs compile-time checks against the AMRN load-region and ABI contracts.
Typed GPIO and peripheral ownership remains explicit in the backend because
the HAL requires compile-time pin types and singleton peripheral ownership.

## Electrical requirements

The SD interface must use the board's correct 3.3 V logic levels. The SD module, wiring, power supply, and chip-select pull-up behavior must be verified on the actual hardware before acceptance testing.

## Clock and logging

The reference kernel targets a 100 MHz system clock and supports both RTT and
USB CDC logging. The F405 backend targets 168 MHz from its 8 MHz HSE. Both
backends configure the USB FS 48 MHz clock domain and use PA11/PA12 for USB
D-/D+.

## Hardware acceptance evidence

Each hardware milestone should record board revision, wiring, firmware revision, SD-card type/filesystem, power source, logging channel, and observed output.
