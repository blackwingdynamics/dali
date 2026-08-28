# Board

The project currently has one compile-time kernel backend. The F405 board is
the MVP execution platform. A BlackPill profile is retained only as a
generator input and has no kernel backend.

## Generator-only board profile

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
