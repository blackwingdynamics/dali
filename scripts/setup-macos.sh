#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly RUST_TOOLCHAIN="1.97.1"
readonly EMBEDDED_TARGET="thumbv7em-none-eabihf"
readonly GIT_CLIFF_VERSION="2.13.0"
export PATH="${CARGO_HOME:-${HOME}/.cargo}/bin:${PATH}"

readonly BREW_PACKAGES=(
    cmake
    dfu-util
    picocom
    pkg-config
    python
    screen
)

log() {
    printf '[setup-macos] %s\n' "$1"
}

fail() {
    printf '[setup-macos] ERROR: %s\n' "$1" >&2
    exit 1
}

require_macos() {
    [[ "$(uname -s)" == "Darwin" ]] || fail "This script supports macOS only."
}

require_repository_root() {
    [[ -f "${REPOSITORY_ROOT}/rust-toolchain.toml" ]] || \
        fail "Run this script from a Dali OS checkout containing rust-toolchain.toml."
}

require_homebrew() {
    command -v brew >/dev/null 2>&1 || \
        fail "Homebrew is required. Install it from https://brew.sh/ and rerun this command."
}

install_brew_packages() {
    log "Installing Homebrew dependencies."
    brew install "${BREW_PACKAGES[@]}"
    brew tap probe-rs/probe-rs
    brew install probe-rs
    brew tap renode/tap
    brew install renode
}

install_rustup() {
    if command -v rustup >/dev/null 2>&1; then
        log "rustup is already installed."
        return
    fi

    log "Installing rustup-init."
    brew install rustup-init
    rustup-init -y --profile minimal --default-toolchain none
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
    probe-rs --version
    renode --version
}

run_repository_check() {
    log "Running the embedded kernel check."
    cd "${REPOSITORY_ROOT}"
    cargo check-kernel
}

main() {
    require_macos
    require_repository_root
    require_homebrew
    install_brew_packages
    install_rustup
    install_rust_toolchain
    install_development_tools
    verify_tools
    run_repository_check
    log "Dali OS macOS development environment is ready."
}

main "$@"
