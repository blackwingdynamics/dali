# `[usb]`

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

[i2c]
bus_frequency_hz = 100_000

[driver_probe]
uart_baud_rate_hz = 115_200
spi_clock_hz = 1_000_000
timeout_ticks = 100
polls_per_timeout_tick = 1
buffer_length = 1
spi_fill_byte = 0xff
spi_device_id = 0
i2c_address = 0x42

[user_key]
port = 'C'
pin = 13

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
