# Reference Hardware

## Board

The project supports two compile-time board backends. The BlackPill remains the
MVP reference platform; the F405 board is the secondary platform for early
on-board SDIO bring-up.

### MVP reference board

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

### Secondary SDIO board

- Board: WeAct Studio STM32F405RGT6 Core Board, 64-pin
- MCU: STM32F405RGT6
- HSE: 25 MHz
- Maximum documented MCU frequency: 168 MHz
- Status LED: PC13, active-low push-pull
- User key: PA0
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
SDIO pin tuple is owned by the board backend and will be transferred to the
storage subsystem when SDIO support is implemented. It is not yet an MVP
package target; AMRN target compatibility remains defined by `AMRN_FORMAT.md`.

## Electrical requirements

The SD interface must use the board's correct 3.3 V logic levels. The SD module, wiring, power supply, and chip-select pull-up behavior must be verified on the actual hardware before acceptance testing.

## Clock and logging

The reference kernel targets a 100 MHz system clock and RTT logging. The F405
backend targets 168 MHz from its 25 MHz HSE. USB CDC is a separate milestone
because it requires a valid USB clock configuration, including the required
48 MHz clock domain.

## Hardware acceptance evidence

Each hardware milestone should record board revision, wiring, firmware revision, SD-card type/filesystem, power source, logging channel, and observed output.
