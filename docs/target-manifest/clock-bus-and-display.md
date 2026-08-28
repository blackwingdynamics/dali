# `[clock]`

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

## `[i2c]`

I2C timing values are target configuration, not driver implementation
constants. The build generates them into the typed `I2cProfile`; a backend
must consume that profile when configuring its peripheral.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `bus_frequency_hz` | integer | yes | Target I2C bus frequency in hertz. |

## `[display]`

The display profile describes an optional I2C OLED capability. The F405 OLED
backend consumes this generated profile and falls back to headless operation
when the configured controller is unavailable. SPI displays and ILI9341
experiments are outside this contract.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `controller` | string | yes | Supported controller identifier, such as `SSD1306` or `SH1106`. |
| `i2c_address` | integer | yes | Seven-bit I2C address used by the display. |
| `width` | integer | yes | Display width in pixels. |
| `height` | integer | yes | Display height in pixels. |
| `text_cell_width` | integer | yes | Text-cell width in pixels used to derive the bounded console columns. |
| `text_cell_height` | integer | yes | Text-cell height in pixels used to derive the bounded console rows. |
| `timeout_ticks` | integer | yes | Maximum timeout for one display operation. |

## `[driver_probe]`

The acceptance probe uses these target-owned values for its UART, SPI, I2C,
timeout, and buffer configuration. They are generated into
`DriverProbeProfile`; the probe implementation must not duplicate them.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `uart_baud_rate_hz` | integer | yes | UART baud rate used by the probe. |
| `spi_clock_hz` | integer | yes | SPI clock used by the probe. |
| `timeout_ticks` | integer | yes | Maximum probe timeout in contract ticks. |
| `polls_per_timeout_tick` | integer | yes | Hardware polls represented by one timeout tick. |
| `timer_timeout_divisor` | integer | yes | Divisor used to derive the timer timeout probe duration. |
| `timer_evidence_poll_limit` | integer | yes | SysTick polling budget used by the timer probe. |
| `buffer_length` | integer | yes | Probe transfer buffer length in bytes. |
| `spi_fill_byte` | integer | yes | Initial byte used by the SPI probe. |
| `spi_device_id` | integer | yes | Logical SPI device identifier used by the probe. |
| `i2c_address` | integer | yes | Logical I2C address used by the probe. |
