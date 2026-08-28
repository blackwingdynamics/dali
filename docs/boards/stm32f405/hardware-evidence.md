# STM32F405 Hardware Evidence

This page summarizes the board-specific evidence boundary. The complete
historical execution records remain in [F405 silicon evidence](../../testing/f405-silicon.md).

## Recorded setup

- Board: WeAct Studio STM32F405RGT6 Core Board
- MCU: STM32F405RGT6
- Reported revision: v1.1 for the I2C/OLED setup
- Power: USB for the reported setup; OLED supplied from 3.3 V
- Debug and console: Raspberry Pi Pico 2 CMSIS-DAP/SWD and USB CDC
- Reported I2C wiring: PB6=SCL, PB7=SDA, common VCC and GND

## Established and open results

The recorded F405 boot, clock, GPIO, SDIO, loader, and security evidence is
listed in the historical record. The I2C scan did not receive an ACK from the
tested devices, and the OLED did not render diagnostics. Therefore I2C/OLED
physical acceptance remains open. Compilation and host tests do not replace
that silicon evidence.

The required next measurement is a signal-level check of SCL and SDA with an
oscilloscope or logic analyzer when suitable laboratory equipment is available.
