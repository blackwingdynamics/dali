# Dali CLI Quickstart

This procedure creates a package from an already linked native payload and
validates the resulting AMRN file.

## 1. Build the application payload

Use the repository application workflow:

~~~text
just app-build
~~~

The input must be a linked native payload for the documented target and load
address. A Rust source file is not a valid input to dali package.

## 2. Create the package

~~~text
dali package \
  --input <payload.bin> \
  --output <application.amrn> \
  --entry-offset <byte-offset>
~~~

The repository demo shortcut is:

~~~text
just package-hello
~~~

## 3. Validate the package

~~~text
dali inspect --input <application.amrn>
~~~

The command must report AMRN package valid before the package is installed on
hardware.

## 4. Continue to hardware

Follow the SD-card and physical acceptance procedures in Workflows. The CLI
validation step does not prove hardware execution.
