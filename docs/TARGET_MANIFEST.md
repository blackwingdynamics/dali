# Target Manifest Reference

This document defines the `targets/*.toml` manifest contract. A manifest is
the single source of truth for board facts and Dali compatibility metadata.
The build script validates every manifest and generates the typed
`dali-targets` registry. The kernel still performs the final typed HAL mapping;
the manifest does not replace compile-time peripheral ownership.

## When to add a manifest

Add one manifest for each board profile. The profile must describe facts from
the board and MCU documentation, not values guessed from another board.

Adding a manifest alone does not add kernel support. An accepted board also
requires a reviewed `kernel/src/board/` backend, target checks, documentation,
and hardware evidence. Do not copy values into CLI commands or ordinary
implementation modules.

## File location and naming

Manifests live directly under the repository `targets/` directory:

```text
targets/<profile-name>.toml
```

The file name is organizational. The stable profile name is the value of
`[profile].name` and is used by CLI commands and application configuration.
Profile names must be unique and should use lowercase kebab-case or lowercase
alphanumeric names consistent with existing commands.

## Sections

Every manifest contains:

- `[profile]` — identity and Dali compatibility;
- `[artifacts]` — optional conventional build artifact names;
- `[profile.dfu]` — optional USB DFU identity and download configuration;
- `[clock]` — oscillator and bus frequencies;
- `[memory]` — kernel, application, and runtime regions;
- `[status_led]` — logical status LED mapping;
- `[usb]` — USB FS controller and data pins.

`[storage]` is optional because a board profile may be described before its
storage transport is supported. Storage must not be omitted from a profile
that claims a kernel storage backend.

## `[profile]`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | Stable CLI/profile identifier. |
| `backend` | string | yes | Stable hardware backend identifier selected by the platform layer. |
| `board` | string | yes | Manufacturer board name. |
| `mcu` | string | yes | Exact MCU identifier. |
| `rust_target` | string | yes | Rust target triple used by the build. |
| `probe_chip` | string | no | Chip identifier accepted by the debug probe tool. Required by probe workflows. |
| `application_supported` | boolean | yes | Whether AMRN applications may target this profile. |
| `amrn_target_id` | integer | yes | Non-zero AMRN target ID when application support is enabled; otherwise `0`. |
| `abi_version` | integer | yes | Non-zero application ABI version when application support is enabled; otherwise `0`. |

`probe_chip` is transport metadata, not the MCU display name. Use the exact
identifier reported by the selected debug tool.

## `[profile.dfu]`

The section is optional for profiles without a documented DFU transport.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `vendor_id` | integer | yes | USB vendor identifier reported by the DFU host tool. |
| `product_id` | integer | yes | USB product identifier reported by the DFU host tool. |
| `address` | integer | yes | Flash address passed to the DFU download operation. |
| `alternate` | integer | yes | DFU alternate interface used for the download. |
| `leave` | boolean | yes | Whether the tool requests leaving DFU mode after download. |

These values identify a transport endpoint only. They do not prove that the
target firmware is valid or that flashing is safe.

## `[artifacts]`

The section is optional for profiles without a supported kernel build route.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kernel_binary` | string | yes when section is present | Conventional raw firmware artifact name under the workspace target directory. |
| `kernel_elf` | string | yes when section is present | Conventional ELF artifact name used by probe flashing. |

The CLI combines this declared file name with the workspace build target and
debug profile directories. It does not embed a board-specific artifact path.

## `[clock]`

All frequency fields are integer hertz values. The source and input frequency
must match the board's clock design.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `source` | string | yes | Clock source name used by the board mapping, such as `HSE` or `HSI`. |
| `input_hz` | integer | yes | Source oscillator frequency in hertz. |
| `system_hz` | integer | yes | Target system/core clock in hertz. |
| `pclk1_hz` | integer | yes | APB1 peripheral clock in hertz. |
| `pclk2_hz` | integer | yes | APB2 peripheral clock in hertz. |
| `usb_hz` | integer | yes | USB clock domain frequency in hertz. |

The manifest records values; the backend remains responsible for configuring
PLL and prescaler registers correctly.

## `[memory]`

All origins are byte addresses and all lengths are byte counts. Regions must
match the linker script and the AMRN/ABI contracts.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kernel_origin` / `kernel_length` | integer | yes | Kernel-reserved RAM region. |
| `application_origin` / `application_length` | integer | yes | RAM region available to the native AMRN payload. |
| `runtime_origin` / `runtime_length` | integer | yes | Kernel runtime and stack region. |
| `flash.origin` / `flash.length` | integer | yes | Kernel flash image region used by the linker. |
| `dma.origin` / `dma.length` | integer | yes | SRAM region that remains available to peripheral DMA. |
| `ccm.origin` / `ccm.length` | integer | no | Optional core-coupled memory for privileged runtime state and stack. |

The planned isolated ABI may add an optional `[memory.isolation]` table:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `code_origin` / `code_length` | integer | together | Aligned application code and read-only data region. |
| `data_origin` / `data_length` | integer | together | Aligned writable data and PSP region. |
| `stack_length` | integer | yes with isolation | PSP stack reservation inside the data region. |
| `peripheral_origin` / `peripheral_length` | integer | together | Aligned ordinary peripheral register region. |
| `bus_fault_origin` / `bus_fault_length` | integer | no | Documented reserved F405 code-region range used only by deterministic BusFault test fixtures. |

An isolation manifest may also declare ordered slots with repeated
`[[memory.isolation.slots]]` tables:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | Stable target-local slot name. |
| `code_origin` / `code_length` | integer | yes | Aligned executable code region for the slot. |
| `data_origin` / `data_length` | integer | yes | Aligned writable data and PSP region for the slot. |
| `stack_length` | integer | yes | PSP reservation inside the slot's data region. |

Slots are ordered by declaration. The first slot must preserve the active
single-application code/data contract until the loader and MPU switch to slot
selection. Slot declarations are metadata only until that implementation is
completed; they do not enable multiple applications or context switching.

When present, the code and data regions must be contiguous, begin at
`application_origin`, and end at the application boundary. They are metadata
for the isolated ABI and do not enable MPU protection by themselves. The
`stack_length` value is copied into ABI v3 packages as the PSP reservation.
The peripheral region must be power-of-two-sized and aligned; it is used to make
ordinary peripheral registers inaccessible to unprivileged applications.

Do not change the application region for the current AMRN v1 contract without
updating `AMRN_FORMAT.md`, `ABI.md`, the linker scripts, tests, and roadmap.

## Pin tables

`[status_led]` and every USB/storage pin use the same pin object:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `port` | string | yes | GPIO port identifier, for example `PB`. |
| `number` | integer | yes | GPIO pin number. |
| `alternate_function` | integer | yes | Alternate-function number; use `0` for ordinary GPIO mode. |
| `active_high` | boolean | yes | Whether logical active means a high electrical output. |

The status LED is expressed as a logical output. Applications must not depend
on the physical polarity.

## `[usb]`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `controller` | string | yes | USB controller owned by the backend. |
| `dm` | pin object | yes | USB D- pin. |
| `dp` | pin object | yes | USB D+ pin. |

The backend must additionally document power, pull-up, interrupt, endpoint
memory, and HAL ownership requirements when it is implemented.

## `[storage]`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `controller` | string | yes | Storage controller name. |
| `bus_width` | integer | yes | Number of active data lines; current registry supports 1 through 4. |
| `clock` | pin object | yes | Storage clock pin. |
| `command` | pin object | yes | Storage command pin. |
| `data` | array of four pin objects | yes | Data pins in controller order. |

The array remains four elements even for a narrower bus so the generated type
has a stable shape. The backend decides which declared lines are active.

## Complete F405 example

The current reference manifest is `targets/f405.toml`:

```toml
[profile]
name = "f405"
board = "WeAct Studio STM32F405RGT6 Core Board"
mcu = "STM32F405RGT6"
rust_target = "thumbv7em-none-eabihf"
probe_chip = "STM32F405RGTx"
dfu = { vendor_id = 0x0483, product_id = 0xDF11 }
application_supported = true
amrn_target_id = 0x02
abi_version = 2

[clock]
source = "HSE"
input_hz = 8_000_000
system_hz = 168_000_000
pclk1_hz = 42_000_000
pclk2_hz = 84_000_000
usb_hz = 48_000_000

[memory]
kernel_origin = 0x2000_0000
kernel_length = 32_768
application_origin = 0x2000_8000
application_length = 65_536
runtime_origin = 0x2001_8000
runtime_length = 32_768

[memory.isolation]
code_origin = 0x2000_8000
code_length = 32_768
data_origin = 0x2001_0000
data_length = 32_768
peripheral_origin = 0x4000_0000
peripheral_length = 536_870_912
bus_fault_origin = 0x0010_0000
bus_fault_length = 32

[status_led]
port = "PB"
number = 2
alternate_function = 0
active_high = true

[usb]
controller = "OTG_FS"
dm = { port = "PA", number = 11, alternate_function = 10, active_high = false }
dp = { port = "PA", number = 12, alternate_function = 10, active_high = false }

[storage]
controller = "SDIO"
bus_width = 4
clock = { port = "PC", number = 12, alternate_function = 12, active_high = false }
command = { port = "PD", number = 2, alternate_function = 12, active_high = false }
data = [
    { port = "PC", number = 8, alternate_function = 12, active_high = false },
    { port = "PC", number = 9, alternate_function = 12, active_high = false },
    { port = "PC", number = 10, alternate_function = 12, active_high = false },
    { port = "PC", number = 11, alternate_function = 12, active_high = false },
]
```

## Validation and generation

From the repository root:

```text
cargo check -p dali-targets
cargo test -p dali-targets -p dali-cli
dali target list
dali target scaffold f405 --output <existing-directory>
```

The scaffold command creates a `.rs.template` and a board review document. It
renders the manifest values as named constants, but it does not register or
compile a backend automatically. Review the generated mapping before adding a
module to `kernel/src/board/mod.rs`.
