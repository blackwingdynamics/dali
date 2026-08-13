# Dali OS build and hardware task runner.
# Run `just` or `just --list` to see the available commands.

set shell := ["bash", "-cu"]

target := "thumbv7em-none-eabihf"
kernel_package := "dali-kernel"
kernel_binary := "dali-kernel"
kernel_elf := "target/" + target + "/debug/" + kernel_binary
kernel_bin := "target/" + target + "/debug/" + kernel_binary + ".bin"
f405_kernel_bin := "target/" + target + "/debug/" + kernel_binary + "-f405.bin"
renode_script := "simulation/renode/dali_blackpill.resc"
chip := env_var_or_default("DALI_CHIP", "STM32F411CEUx")
f405_chip := env_var_or_default("DALI_F405_CHIP", "STM32F405RGTx")
dfu_device := env_var_or_default("DALI_DFU_DEVICE", "0483:df11")
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
kernel-check board="f411":
    cargo check -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd" } else if board == "f411" { "board-blackpill-f411" } else { error("Unsupported board. Use f411 or f405.") } }} --target {{target}}

# Run host-side tests.
test:
    cargo test -p dali-sdk -p dali-app-hello -p dali-cli

# Run host-side Clippy with warnings denied.
clippy:
    cargo clippy --workspace --all-targets --exclude {{kernel_package}} -- -D warnings

# Run kernel-target Clippy with warnings denied.
kernel-clippy board="f411":
    cargo clippy -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd" } else if board == "f411" { "board-blackpill-f411" } else { error("Unsupported board. Use f411 or f405.") } }} --target {{target}} --bin {{kernel_binary}} -- -D warnings

# Run all local CI checks.
ci: format-check workspace-check kernel-check test clippy kernel-clippy diff-check

# Build the embedded kernel ELF.
build board="f411":
    cargo build -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd" } else if board == "f411" { "board-blackpill-f411" } else { error("Unsupported board. Use f411 or f405.") } }} --target {{target}}

# Run the kernel in Renode. Requires Renode and uses the STM32F4 reference model.
simulate board="f411":
    just build {{board}}
    renode --console --disable-gui {{renode_script}}

# Convert the kernel ELF to a raw binary. Requires cargo-binutils and llvm-tools.
bin board="f411":
    just build {{board}}
    cargo objcopy -p {{kernel_package}} --no-default-features --features {{ if board == "f405" { "board-stm32f405-sd" } else if board == "f411" { "board-blackpill-f411" } else { error("Unsupported board. Use f411 or f405.") } }} --target {{target}} --bin {{kernel_binary}} -- -O binary {{ if board == "f405" { f405_kernel_bin } else { kernel_bin } }}

# Flash the kernel with a connected probe. Requires probe-rs.
flash-probe board="f411":
    just build {{board}}
    {{ if board == "f405" { "probe-rs run --chip " + f405_chip + " " + kernel_elf } else if board == "f411" { "probe-rs run --chip " + chip + " " + kernel_elf } else { error("Unsupported board. Use f411 or f405.") } }}

# Flash the raw binary through STM32 DFU mode. Requires dfu-util and host permissions.
flash-dfu board="f411":
    just bin {{board}}
    dfu-util -d {{dfu_device}} -a 0 -s {{flash_address}}:leave -D {{ if board == "f405" { f405_kernel_bin } else if board == "f411" { kernel_bin } else { error("Unsupported board. Use f411 or f405.") } }}

# Attach to a running target and stream supported debug output.
attach:
    probe-rs attach --chip {{chip}}

# Check committed and working-tree whitespace errors.
diff-check:
    git diff --check

# Remove Cargo build artifacts.
clean:
    cargo clean
