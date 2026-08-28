# Dali CLI Quickstart

This procedure creates a Dali cartridge from an already linked native payload
and validates the resulting AMRN file.

## 1. Build the application payload

Use the repository application workflow:

~~~text
just app-build
~~~

The input must be a linked native payload for the documented target and load
address. A Rust source file is not a valid input to `dali cartridge`.

## 2. Create the cartridge

~~~text
dali cartridge \
  --input <payload.bin> \
  --output <application.amrn> \
  --entry-offset <byte-offset>
~~~

The repository demo shortcut is:

~~~text
just cartridge-hello
~~~

## 3. Validate the cartridge

~~~text
dali inspect --input <application.amrn>
~~~

The command must report AMRN cartridge valid before the cartridge is installed
on hardware.

## 4. Continue to hardware

Follow the SD-card and physical acceptance procedures in Workflows. The CLI
validation step does not prove hardware execution.
