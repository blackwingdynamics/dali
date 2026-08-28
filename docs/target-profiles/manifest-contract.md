# Manifest contract

Each supported target manifest contains these sections:

| Section | Purpose |
| --- | --- |
| `profile` | Stable profile name, board name, MCU, Rust target, probe identifier, application support, AMRN target ID, and ABI version |
| `capabilities` | Explicit storage, USB console, MPU, and relocation support declarations |
| `clock` | Clock source, input, system, APB1, APB2, and USB frequencies in hertz |
| `memory` | Kernel, application, runtime, and optional isolated code/data SRAM regions |
| `status_led` | Logical status LED port, pin, alternate function, and polarity |
| `usb` | USB controller and D-/D+ pins with alternate functions |
| `storage` | Optional storage controller, bus width, clock, command, and data pins with alternate functions |

Values in this manifest are configuration and board-definition data, not
implementation literals. `application_supported = true` requires non-zero
AMRN and ABI identifiers. Profiles used by probe workflows declare their chip
identifier in the manifest. A board profile can remain metadata-only while its
kernel mapping is pending. Adding an accepted target requires a new manifest,
a kernel backend mapping, target checks, and the relevant hardware
documentation and acceptance evidence.
