#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly RUST_TOOLCHAIN="1.97.1"
readonly EMBEDDED_TARGET="thumbv7em-none-eabihf"
readonly GIT_CLIFF_VERSION="2.13.0"
readonly RENODE_PACKAGE_URL="https://builds.renode.io/renode-latest.pkg.tar.xz"
readonly PROBE_RULES_URL="https://probe.rs/files/69-probe-rs.rules"
export PATH="${CARGO_HOME:-${HOME}/.cargo}/bin:${PATH}"

readonly PACMAN_PACKAGES=(
    base-devel
    cmake
    curl
    dfu-util
    dotnet-runtime
    git
    picocom
    pkgconf
    python
    probe-rs
    rustup
    screen
    usbutils
)

log() {
    printf '[setup-arch] %s\n' "$1"
}

fail() {
    printf '[setup-arch] ERROR: %s\n' "$1" >&2
    exit 1
}

require_arch() {
    [[ -r /etc/arch-release ]] || fail "This script supports Arch Linux only."
}

require_repository_root() {
    [[ -f "${REPOSITORY_ROOT}/rust-toolchain.toml" ]] || \
        fail "Run this script from a Dali OS checkout containing rust-toolchain.toml."
}

install_system_packages() {
    log "Installing Arch host dependencies."
    sudo -v
    sudo pacman -S --needed "${PACMAN_PACKAGES[@]}"
    sudo usermod --append --groups uucp "${USER}"
    log "Added ${USER} to the uucp group for USB serial access."
}

install_renode() {
    if command -v renode >/dev/null 2>&1; then
        log "Renode is already installed."
        return
    fi

    local download_directory
    download_directory="$(mktemp -d)"

    log "Installing the latest Renode Arch package."
    curl --fail --location --silent --show-error \
        "${RENODE_PACKAGE_URL}" \
        --output "${download_directory}/renode.pkg.tar.xz"
    sudo pacman -U --needed "${download_directory}/renode.pkg.tar.xz"
    rm -rf -- "${download_directory}"
}

install_probe_rules() {
    local rules_file
    rules_file="$(mktemp)"

    log "Installing probe-rs udev rules."
    curl --fail --location --silent --show-error \
        "${PROBE_RULES_URL}" \
        --output "${rules_file}"
    sudo install -Dm644 "${rules_file}" /etc/udev/rules.d/69-probe-rs.rules
    rm -f -- "${rules_file}"
    sudo udevadm control --reload-rules
    sudo udevadm trigger

    if ! getent group plugdev >/dev/null 2>&1; then
        sudo groupadd --system plugdev
    fi
    sudo usermod --append --groups plugdev "${USER}"
    log "The plugdev group may require a logout and login before probe access works."
}

install_rust_toolchain() {
    log "Installing the pinned Rust toolchain and embedded target."
    rustup toolchain install "${RUST_TOOLCHAIN}" \
        --profile minimal \
        --component clippy \
        --component llvm-tools-preview \
        --component rustfmt \
        --target "${EMBEDDED_TARGET}"
}

install_cargo_tool() {
    local package_name="$1"
    local command_name="$2"
    local version_argument="${3:-}"

    if command -v "${command_name}" >/dev/null 2>&1; then
        log "${command_name} is already installed."
        return
    fi

    log "Installing ${package_name} with Cargo."
    if [[ -n "${version_argument}" ]]; then
        cargo install "${package_name}" --version "${version_argument}" --locked
    else
        cargo install "${package_name}" --locked
    fi
}

install_development_tools() {
    install_cargo_tool cargo-binutils cargo-objcopy
    install_cargo_tool just just
    install_cargo_tool lefthook lefthook
    install_cargo_tool git-cliff git-cliff "${GIT_CLIFF_VERSION}"

    log "Installing Lefthook hooks."
    cd "${REPOSITORY_ROOT}"
    lefthook install
}

verify_tools() {
    local required_commands=(
        cargo
        cargo-objcopy
        dfu-util
        git
        git-cliff
        just
        lefthook
        picocom
        probe-rs
        renode
        rustc
        rustup
    )

    log "Verifying installed commands."
    for command_name in "${required_commands[@]}"; do
        command -v "${command_name}" >/dev/null 2>&1 || \
            fail "Required command is not available: ${command_name}"
    done

    rustup run "${RUST_TOOLCHAIN}" rustc --version
    rustup target list --installed --toolchain "${RUST_TOOLCHAIN}" | \
        grep -Fxq "${EMBEDDED_TARGET}" || \
        fail "Embedded target is not installed: ${EMBEDDED_TARGET}"
    dotnet --list-runtimes | grep -Fq 'Microsoft.NETCore.App' || \
        fail ".NET runtime is not available to Renode"
    probe-rs --version
    renode --version
}

run_repository_check() {
    log "Running the embedded kernel check."
    cd "${REPOSITORY_ROOT}"
    cargo check-kernel
}

main() {
    require_arch
    require_repository_root
    install_system_packages
    install_renode
    install_probe_rules
    install_rust_toolchain
    install_development_tools
    verify_tools
    run_repository_check
    log "Dali OS Arch development environment is ready."
}

main "$@"
