# Target tests

Target tests validate one selected backend on its real board. The test plan
must be derived from the typed target profile; values such as clock rates,
memory addresses, pins, timeouts, and transport settings must not be copied
into a generic test procedure.

## Required evidence for every backend

Record the backend ID, target profile, source and firmware revision, board
revision, power source, wiring, transport, expected output, observed output,
and result. A successful build, flash, simulator run, or device enumeration is
not hardware acceptance by itself.

Each backend checkpoint must include:

- boot banner and reset-cause behavior;
- target-profile clock initialization;
- backend-owned status LED and input behavior, when declared;
- every declared storage, serial, USB, I2C, SPI, timer, watchdog, and
  interrupt capability;
- loader and cartridge behavior supported by the selected profile;
- memory and entry-point behavior within the profile's declared regions;
- deterministic application or service evidence for each enabled path;
- bounded timeout, error, recovery, and unavailable-capability paths;
- comparison with the previous accepted trace after every structural change.

## F405 reference profile

The current F405 target additionally records the accepted reference behavior:

- 168 MHz system clock initialization;
- board-specific status LED behavior;
- hardware SDIO initialization and one known block read;
- FAT16/FAT32 root-directory enumeration;
- `.AMRN` extension filtering and arbitrary cartridge-name discovery;
- payload loading into the profile-declared application SRAM region;
- demo cartridge entry and deterministic application LED pattern.

F405 I2C device ACK and physical OLED rendering remain separate open hardware
acceptance items. They must not be marked complete from host tests or from a
successful target boot alone.

## Backend isolation check

The selected target must build without linking an unselected backend. CI and
target validation must reject direct cross-backend imports, duplicate target
identifiers, ambiguous capability profiles, and hardware values that bypass
typed target metadata.
