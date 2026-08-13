# Dali OS Simulation

Renode is the supported development simulator for the first embedded bring-up
stage. The simulation uses Renode's STM32F4 Discovery platform as the closest
available reference model for the STM32F411 BlackPill target.

## Run

From the repository root:

```text
just simulate
just simulate f405
```

The optional board argument selects only the firmware build. The scenario uses
one generic Renode STM32F4 reference machine for both board variants; the
machine name is not a chip or board selection mechanism.

## Evidence boundary

Simulation can provide development evidence for CPU boot, linker placement,
SysTick progress, and selected GPIO behavior. It does not prove BlackPill
electrical behavior, the exact STM32F411 clock tree, SD-card timing, SPI signal
integrity, or physical LED behavior.

The current kernel logger writes to RTT. The initial Renode scenario therefore
does not promise visible kernel logs. A future simulation logging backend may
route the same logging facade to a Renode virtual UART without changing the
production RTT backend. USB CDC is deliberately disabled by `just simulate`:
the selected Renode STM32F4 reference model does not implement the OTG_FS
global register block used by the production USB backend. Running that backend
in the model produces unsupported-register warnings and does not provide
evidence about physical USB enumeration.
