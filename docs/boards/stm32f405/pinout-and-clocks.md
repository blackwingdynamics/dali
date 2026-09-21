# STM32F405 Pinout and Clocks

## Board identity

- Board: WeAct Studio STM32F405RGT6 Core Board, 64-pin
- MCU: STM32F405RGT6
- Reported board revision for the I2C/OLED setup: v1.1
- Target triple: `thumbv7em-none-eabihf`

## Clock and control signals

- HSE: 8 MHz
- System clock target: 168 MHz
- Status LED: PB2, active-high
- User key: PC13, active-low with pull-up
- SWD: SWDIO, SWCLK, and GND

## Peripheral mapping

| Function | Signal |
| --- | --- |
| I2C1 SCL | PB6 |
| I2C1 SDA | PB7 |
| SDIO clock | PC12 |
| SDIO command | PD2 |
| SDIO data 0 | PC8 |
| SDIO data 1 | PC9 |
| SDIO data 2 | PC10 |
| SDIO data 3 | PC11 |
| USB D- / D+ | PA11 / PA12 |

The OLED setup was powered from 3.3 V and shared board ground. These are
reported wiring facts for the recorded setup; verify them against the physical
board and module documentation before acceptance.

Target-specific values are declared by `targets/f405.toml`; compile-time pin
and peripheral ownership remains in the F405 backend.
