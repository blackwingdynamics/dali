#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly RUST_TOOLCHAIN="1.97.1"
readonly EMBEDDED_TARGET="thumbv7em-none-eabihf"
readonly GIT_CLIFF_VERSION="2.13.0"
readonly RENODE_ARCHIVE_URL="https://builds.renode.io/renode-latest.linux-portable.tar.gz"
readonly PROBE_RULES_URL="https://probe.rs/files/69-probe-rs.rules"
readonly RENODE_INSTALL_DIRECTORY="${HOME}/.local/opt/renode"
readonly CARGO_BIN_DIRECTORY="${CARGO_HOME:-${HOME}/.cargo}/bin"
export PATH="${CARGO_BIN_DIRECTORY}:${PATH}"

readonly DNF_PACKAGES=(
    ca-certificates
    cmake
    curl
    dfu-util
    gcc
    gcc-c++
    git
    libicu
    libudev-devel
    libusb1-devel
    make
    picocom
    pkgconf-pkg-config
    python3
    rustup
    screen
    usbutils
)

log() {
    printf '[setup-fedora] %s\n' "$1"
}

fail() {
    printf '[setup-fedora] ERROR: %s\n' "$1" >&2
    exit 1
}

require_fedora() {
    [[ -r /etc/fedora-release ]] || fail "This script supports Fedora only."
}

require_repository_root() {
    [[ -f "${REPOSITORY_ROOT}/rust-toolchain.toml" ]] || \
        fail "Run this script from a Dali OS checkout containing rust-toolchain.toml."
}

install_system_packages() {
    local dnf_command
    dnf_command="$(command -v dnf5 || command -v dnf || true)"
    [[ -n "${dnf_command}" ]] || fail "Neither dnf5 nor dnf is available."

    log "Installing Fedora host dependencies."
    sudo -v
    sudo "${dnf_command}" install --assumeyes "${DNF_PACKAGES[@]}"
}

install_renode() {
    if command -v renode >/dev/null 2>&1; then
        log "Renode is already installed."
        return
    fi

    local download_directory
    download_directory="$(mktemp -d)"

    log "Installing the portable Renode release."
    curl --fail --location --silent --show-error \
        "${RENODE_ARCHIVE_URL}" \
        --output "${download_directory}/renode.tar.gz"
    mkdir -p "${RENODE_INSTALL_DIRECTORY}"
    tar --extract --gzip --file "${download_directory}/renode.tar.gz" \
        --directory "${RENODE_INSTALL_DIRECTORY}" --strip-components=1
    ln -sfn "${RENODE_INSTALL_DIRECTORY}/renode" "${CARGO_BIN_DIRECTORY}/renode"
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
    install_cargo_tool probe-rs-tools probe-rs

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
    probe-rs --version
    renode --version
}

run_repository_check() {
    log "Running the embedded kernel check."
    cd "${REPOSITORY_ROOT}"
    cargo check-kernel
}

main() {
    require_fedora
    require_repository_root
    install_system_packages
    install_renode
    install_probe_rules
    install_rust_toolchain
    install_development_tools
    verify_tools
    run_repository_check
    log "Dali OS Fedora development environment is ready."
}

main "$@"
