# `dali device info`

## Purpose

Display one host-visible device and the generated Dali target metadata matched
to that device. This command is read-only.

## Usage

```text
dali device list
dali device info <id-or-path>
```

Use the `id` shown by `dali device list`. A transport path can also be used
when the selected record exposes one, for example `/dev/ttyACM0` for a CDC
console.

The command discovers the supported transports, selects exactly one matching
record, and prints the normalized device fields. If the record is matched to a
declared target profile, the complete generated profile metadata is printed
under `target metadata:`. An unidentified device is reported without guessed
board information.

The command does not reset, attach, flash, open a terminal, or change device
state.

## Example

When the F405 board is in DFU mode:

~~~text
$ dali device info 0483:df11
dfu device: DFU device
  id: 0483:df11
  state: available
  target: f405
  vendor: 0483
  serial: 357135693234
  path: 1-10.1
  capabilities: flash

target metadata:
profile: f405
board: WeAct Studio STM32F405RGT6 Core Board
mcu: STM32F405RGT6
rust_target: thumbv7em-none-eabihf
probe_chip: STM32F405RGTx
dfu: 0x0483:0xDF11
application_supported: true
amrn_target_id: 0x02
abi_version: 2
clock: HSE, input 8000000 Hz, system 168000000 Hz, APB1 42000000 Hz, APB2 84000000 Hz, USB 48000000 Hz
memory: kernel 0x20000000+32768, application 0x20008000+65536, runtime 0x20018000+32768
status_led: PB2 AF0 active-high
usb: OTG_FS, D- PA11 AF10 active-low, D+ PA12 AF10 active-low
storage: SDIO (4-bit), clock PC12 AF12 active-low, command PD2 AF12 active-low, data [PC8 AF12 active-low, PC9 AF12 active-low, PC10 AF12 active-low, PC11 AF12 active-low]
~~~

For a runtime CDC device, target metadata may be unavailable because the
firmware does not declare a target identity through USB CDC.
