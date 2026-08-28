# `[capabilities]`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `storage` | boolean | yes | Whether the selected backend provides the declared storage path. |
| `usb_console` | boolean | yes | Whether the target supports Dali USB CDC console operation. |
| `mpu` | boolean | yes | Whether the target/backend supports the declared MPU protection boundary. |
| `relocation` | boolean | yes | Whether the target supports the declared relocation package contract. |

The build validates these fields against the rest of the manifest. For
example, `storage = true` requires a `[storage]` section, and `mpu` or
`relocation` requires `[memory.isolation]`. Capability declarations describe
support; they do not replace kernel backend implementation or hardware evidence.

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
