# Reference Hardware

## Board

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

## Electrical requirements

The SD interface must use the board's correct 3.3 V logic levels. The SD module, wiring, power supply, and chip-select pull-up behavior must be verified on the actual hardware before acceptance testing.

## Clock and logging

The kernel currently targets a 100 MHz system clock and RTT logging. USB CDC is a separate milestone because it requires a valid USB clock configuration, including the required 48 MHz clock domain.

## Hardware acceptance evidence

Each hardware milestone should record board revision, wiring, firmware revision, SD-card type/filesystem, power source, logging channel, and observed output.
