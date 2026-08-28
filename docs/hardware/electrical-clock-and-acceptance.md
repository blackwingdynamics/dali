# Electrical requirements

The SD interface must use the board's correct 3.3 V logic levels. The SD module, wiring, power supply, and chip-select pull-up behavior must be verified on the actual hardware before acceptance testing.

## Clock and logging

The reference kernel targets the F405 168 MHz system clock from its 8 MHz HSE
and supports both RTT and USB CDC logging. The backend configures the USB FS
48 MHz clock domain and uses PA11/PA12 for USB D-/D+.

## Hardware acceptance evidence

Each hardware milestone should record board revision, wiring, firmware revision, SD-card type/filesystem, power source, logging channel, and observed output.
