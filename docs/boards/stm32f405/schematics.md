# STM32F405 Wiring Record

This is a textual wiring record, not a replacement for an official board or
module schematic.

| Connection | Wiring record |
| --- | --- |
| SWD | SWDIO, SWCLK, GND |
| I2C1 | PB6=SCL, PB7=SDA, 3.3 V, GND |
| SDIO clock | PC12 |
| SDIO command | PD2 |
| SDIO data | PC8, PC9, PC10, PC11 |
| USB console | PA11=D-, PA12=D+ |

The reported OLED setup used 3.3 V power and a shared ground. See [pinout and
clocks](pinout-and-clocks.md) for the canonical signal mapping and [hardware
evidence](hardware-evidence.md) for what has and has not been verified.
