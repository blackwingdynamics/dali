# STM32F405 Hardware Evidence

This page summarizes the board-specific evidence boundary. The complete
historical execution records remain in [F405 silicon evidence](../../testing/f405-silicon.md).

## Recorded setup

- Board: WeAct Studio STM32F405RGT6 Core Board
- MCU: STM32F405RGT6
- CPU: Arm Cortex-M4 with FPU and DSP extensions, up to 168 MHz
- On-chip memory: up to 1 MiB Flash, up to 192 KiB SRAM, plus 4 KiB backup SRAM
- Relevant capabilities: MPU, 16-channel DMA, SDIO, USB OTG FS/HS, two CAN
  interfaces, FSMC, three SPI interfaces, and three I2C interfaces
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

## Flash capacity and cartridge planning

The STM32F405RGT6 device provides up to 1 MiB of internal Flash. The current
`f405` target profile intentionally exposes only the first 512 KiB to the Dali
firmware layout: 384 KiB for firmware and the final 128 KiB erase sector for
the Flash artifact store. This is a conservative software layout, not the
physical Flash limit of the MCU.

The artifact sector is dedicated to cartridge storage. The current logical
artifact capacity is 64 KiB for one replaceable AMRN cartridge, with the
remaining sector space available for publication metadata and future storage
layout work. A future two-slot AMRN design may use the unused Flash sectors,
but it requires an explicit target-manifest, linker, writer, recovery, and
validation change before implementation.

## Latest F405 regression record

- Date: 2026-08-29
- Firmware source revision: `0cfa1dd`
- Board and MCU: WeAct Studio STM32F405RGT6 Core Board, STM32F405RGT6
- Power: USB
- Flash transport: Raspberry Pi Pico 2 CMSIS-DAP/SWD at 1000 kHz
- Console transport: the board's USB CDC console, selected through `dali device list`
- Storage: the connected SD card with Binary v2 generation `version=1`, `sequence=2`, `slot=B`
- Expected result: one complete repository load, signed AMRN validation, and one application log
- Observed result: SDIO initialization, Root/Bundle/Timestamp/Snapshot/Revocations loading,
  trust-state reconstruction, AMRN signature verification, slot-0 placement, and exactly
  one `[INFO][APP] Hello World from AMRN` line
- Result: passed for this F405 boot, storage, signed-cartridge, and USB CDC path
- Limitation: this record does not establish I2C/OLED acceptance, Secure Boot, arbitrary
  DMA isolation, or multi-application isolation
