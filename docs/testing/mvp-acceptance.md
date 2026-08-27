# MVP acceptance test

The MVP passes only when a freshly flashed kernel discovers an `.amrn` package on the SD card, validates its 32-byte header and CRC32, loads it into the reserved SRAM region, transfers control to `unsafe extern "C" fn(*const ServiceTable) -> !`, produces the documented application LED pattern, and delivers the application log messages on hardware.

Build success, parser tests, or a simulated jump do not independently prove the end-to-end milestone.
