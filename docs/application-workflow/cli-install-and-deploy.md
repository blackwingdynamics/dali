# Install the CLI

Install the host CLI from the repository with Cargo:

```text
cargo install --path crates/dali-cli --locked
```

After installation, the user-facing executable is `dali`:

```text
dali inspect --input target/thumbv7em-none-eabihf/debug/hello.amrn
```

The Cargo cartridge remains `dali-cli`; `dali` is the installed executable name.

## Install a cartridge on the SD card

1. Build the cartridge with `just cartridge-hello`.
2. Insert the SD card into the host card reader.
3. Identify the card device with a size-aware command such as:

   ```text
   lsblk -o NAME,PATH,SIZE,FSTYPE,LABEL,MOUNTPOINTS,TRAN
   ```

4. Mount the card's existing FAT partition using the device path reported by
   the host. Do not guess the device name.
5. Copy the generated `.amrn` file into the filesystem root:

   ```text
   cp target/thumbv7em-none-eabihf/debug/hello.amrn <mounted-sd-root>/hello.amrn
   sync
   ```

6. Unmount the card cleanly before removing it:

   ```text
   umount <mounted-sd-root>
   ```

The kernel scans the root directory for `.amrn` files and does not require the
basename `hello`. The current MVP policy requires exactly one root AMRN file;
zero files and multiple files are reported as cartridge-selection failures.

## Flash and observe loader validation

Flash the kernel for the selected board using the board-specific repository
recipe. For the STM32F405 board, for example:

```text
just flash-probe f405
```

Open the USB CDC console after the runtime device appears:

```text
just console
```

Successful cartridge discovery and validation currently produce log records
equivalent to:

```text
[STORAGE] Read block 0 successfully
[LOADER] AMRN header and payload validated
```

This proves that the board read the card, found one AMRN file, validated its
header and exact length, and verified its payload CRC32. It does not yet prove
that the payload was copied to SRAM or executed.
