# Boards and memory

The project currently has one implemented compile-time kernel backend. The
F405 board is the MVP reference execution platform, not the definition of the
kernel architecture. Each future board must provide its own platform backend
directory and typed target profile; its hardware facts must not be copied into
shared kernel policy.

The backend owns the board's clock tree, pins, peripherals, interrupt wiring,
linker layout, DMA placement, and processor-specific memory rules. The kernel
core consumes only the typed capabilities exposed by the selected profile.

## Generator-only board profile

- Board: WeAct BlackPill
- MCU: STM32F411CEU6
- Core: ARM Cortex-M4F
- Frequency target: 100 MHz
- Application target: not supported by the current kernel backend

This profile remains generator-only until a separate F411 backend is
implemented and passes its own target and hardware evidence gates.

## Future board profiles

Pico, FPGA SoC, and other MCU boards must be represented by independent target
profiles and backend directories. A new profile must declare its CPU,
architecture, memory regions, clocks, capabilities, and linker requirements.
Adding it must not require changes to existing board backends or shared kernel
policy modules.

## SRAM layout

```text
0x20000000 - 0x20007FFF   Kernel reserved RAM
0x20008000 - 0x20017FFF   Application region, 64 KiB
0x20018000 - 0x2001FFFF   Kernel runtime and stack RAM
```

The application load address is fixed at `0x20008000` for the F405 MVP
contract. Other targets must declare their own compatible application region
or explicitly provide a different cartridge/ABI contract; the value must not
be assumed by a hardware-neutral module.
