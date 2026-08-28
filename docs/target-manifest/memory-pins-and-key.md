# `[memory]`

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
| `id` | integer | yes | Stable target-manifest slot identifier used by AMRN selection metadata. |
| `name` | string | yes | Stable target-local slot name. |
| `code_origin` / `code_length` | integer | yes | Aligned executable code region for the slot. |
| `data_origin` / `data_length` | integer | yes | Aligned writable data and PSP region for the slot. |
| `stack_length` | integer | yes | PSP reservation inside the slot's data region. |

The first declared slot must preserve the active single-application code/data
contract until the loader and MPU switch to slot selection. The explicit `id`
is the package-facing identity and must remain stable if declarations are
reordered. Slot declarations are metadata and do not by themselves enable
multiple applications or context switching. The feature-gated ABI v3
scheduler and F405 backend provide the separately validated context/MPU
switching path; complete production lifecycle and ownership policy remain
outside this manifest.

When present, the code and data regions must be contiguous, begin at
`application_origin`, and end at the application boundary. They are metadata
for the isolated ABI and do not enable MPU protection by themselves. The
`stack_length` value is copied into ABI v3 packages as the PSP reservation.
The peripheral region must be power-of-two-sized and aligned; it is used to make
ordinary peripheral registers inaccessible to unprivileged applications.

Do not change the application region for the current AMRN v1 contract without
updating `amrn-format/README.md`, `ABI.md`, the linker scripts, tests, and roadmap.

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

## `[user_key]`

The user-key mapping is generated into `UserKeyProfile` and consumed by the
board-owned EXTI binding.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `port` | string | yes | Single GPIO port letter. |
| `pin` | integer | yes | GPIO pin number. |
