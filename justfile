# Dali OS build and hardware task runner.
# Run `just` or `just --list` to see the available commands.

set shell := ["bash", "-cu"]

target := "thumbv7em-none-eabihf"
kernel_package := "dali-kernel"
kernel_binary := "dali-kernel"
kernel_elf := "target/" + target + "/debug/" + kernel_binary
f405_kernel_bin := "target/" + target + "/debug/" + kernel_binary + "-f405.bin"
app_package := "dali-app-hello"
app_binary := "dali-app-hello"
app_payload := "target/" + target + "/debug/" + app_binary + ".bin"
app_package_file := "target/" + target + "/debug/hello.amrn"
app_entry_offset := "0"
renode_script := "simulation/renode/dali_blackpill.resc"
dfu_device := env_var_or_default("DALI_DFU_DEVICE", "0483:df11")
usb_console_port := env_var_or_default("DALI_USB_CONSOLE_PORT", "")
usb_console_baud := "115200"
usb_console_retry_delay := "1"
flash_address := "0x08000000"

default:
    @just --list

# Install the complete Arch Linux development environment.
setup-arch:
    bash scripts/setup-arch.sh

# Install the complete Debian-based Linux development environment.
setup-debian:
    bash scripts/setup-debian.sh

# Install the complete Fedora development environment.
setup-fedora:
    bash scripts/setup-fedora.sh

# Install the complete macOS development environment through Homebrew.
setup-macos:
    bash scripts/setup-macos.sh

# Install the complete Windows development environment through PowerShell.
setup-windows:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/setup-windows.ps1

# Apply rustfmt to the entire workspace.
format:
    cargo fmt --all

# Check formatting without modifying files.
format-check:
    cargo fmt --all -- --check

# Check host-side workspace members.
workspace-check:
    cargo check --workspace --exclude {{kernel_package}}

# Check the embedded kernel target.
kernel-check board="f405":
    cargo check -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd,usb-cdc" } else { error("Unsupported board. Use f405.") } }} --target {{target}}

# Run host-side tests.
test:
    cargo test -p dali -p dali-app-hello -p dali-cli

# Run host-side Clippy with warnings denied.
clippy:
    cargo clippy --workspace --all-targets --exclude {{kernel_package}} -- -D warnings

# Run kernel-target Clippy with warnings denied.
kernel-clippy board="f405":
    cargo clippy -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd,usb-cdc" } else { error("Unsupported board. Use f405.") } }} --target {{target}} --bin {{kernel_binary}} -- -D warnings

# Run all local CI checks.
ci: format-check workspace-check kernel-check test clippy kernel-clippy diff-check

# Build the embedded kernel ELF.
build board="f405":
    cargo build -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd,usb-cdc" } else { error("Unsupported board. Use f405.") } }} --target {{target}}

# Run the kernel in Renode. Requires Renode and uses the STM32F4 reference model.
simulate board="f405":
    cargo build -p {{kernel_package}} --no-default-features --features board-stm32f405-sd --target {{target}}
    renode --console --disable-gui {{renode_script}}

# Convert the kernel ELF to a raw binary. Requires cargo-binutils and llvm-tools.
bin board="f405":
    just build {{board}}
    cargo objcopy -p {{kernel_package}} --no-default-features --features board-stm32f405-sd,usb-cdc --target {{target}} --bin {{kernel_binary}} -- -O binary {{f405_kernel_bin}}

# Build the native demo payload for the documented application SRAM region.
app-build:
    cargo build -p {{app_package}} --features embedded-payload --target {{target}}
    cargo objcopy -p {{app_package}} --features embedded-payload --target {{target}} --bin {{app_binary}} -- -O binary {{app_payload}}

# Assemble the native demo payload into a contract-valid AMRN package.
package-hello: app-build
    cargo run -p dali-cli --bin dali -- package --input {{app_payload}} --output {{app_package_file}} --entry-offset {{app_entry_offset}}

# Flash the kernel with a connected probe. Requires probe-rs.
flash-probe board="f405":
    just build {{board}}
    probe-rs run --chip "$(cargo run --quiet -p dali-cli --bin dali -- target info {{board}} --field probe-chip)" {{kernel_elf}}

# Flash the raw binary through STM32 DFU mode. Requires dfu-util and host permissions.
flash-dfu board="f405":
    just bin {{board}}
    dfu-util -d {{dfu_device}} -a 0 -s {{flash_address}}:leave -D {{f405_kernel_bin}}

# Open the USB CDC runtime console. Requires picocom.
console port=usb_console_port:
    bash scripts/console.sh "{{port}}" "{{usb_console_baud}}" "{{usb_console_retry_delay}}"

# Attach to a running target and stream supported debug output.
attach board="f405":
    probe-rs attach --chip "$(cargo run --quiet -p dali-cli --bin dali -- target info {{board}} --field probe-chip)"

# Check committed and working-tree whitespace errors.
diff-check:
    git diff --check

# Check internal Markdown links and documented repository paths.
docs-check:
    python3 scripts/check-docs.py

# Apply conservative formatting to Markdown documentation.
docs-format:
    python3 scripts/format-docs.py

# Check Markdown formatting without modifying files.
docs-format-check:
    python3 scripts/format-docs.py --check

# Remove Cargo build artifacts.
clean:
    cargo clean
