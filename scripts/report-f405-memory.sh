#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPOSITORY_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly EMBEDDED_TARGET="thumbv7em-none-eabihf"
readonly KERNEL_ELF="${REPOSITORY_ROOT}/target/${EMBEDDED_TARGET}/release/dali-kernel"
readonly PRODUCTION_FEATURES="board-stm32f405-sd,usb-cdc,abi-context-switch,abi-relocation,abi-authentication,repository-loader,storage-write"
readonly REPORT_DIR="${DALI_REPORT_DIR:-${REPOSITORY_ROOT}/target/f405-memory-report}"
readonly MAP_FILE="${REPORT_DIR}/dali-f405-memory.map"

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        printf 'ERROR: required command is missing: %s\n' "$1" >&2
        exit 1
    }
}

find_llvm_tools() {
    local sysroot
    local llvm_size
    sysroot="$(rustc --print sysroot)"
    llvm_size="$(find "${sysroot}" -type f -name llvm-size -print -quit)"
    [[ -n "${llvm_size}" ]] || {
        printf 'ERROR: llvm-tools-preview does not provide llvm-size\n' >&2
        exit 1
    }
    dirname -- "${llvm_size}"
}

build_kernel() {
    local existing_flags="${RUSTFLAGS:-}"
    mkdir -p -- "${REPORT_DIR}"
    CARGO_PROFILE_RELEASE_DEBUG=2 \
        RUSTFLAGS="${existing_flags} -C link-arg=-Tlink.x -C link-arg=-Map=${MAP_FILE}" \
        cargo build -p dali-kernel --release --no-default-features \
            --features "${PRODUCTION_FEATURES}" \
            --target "${EMBEDDED_TARGET}"
}

write_report() {
    local llvm_tools_dir="$1"
    "${llvm_tools_dir}/llvm-size" -A "${KERNEL_ELF}" > "${REPORT_DIR}/llvm-size.txt"
    "${llvm_tools_dir}/llvm-nm" --print-size --size-sort "${KERNEL_ELF}" > "${REPORT_DIR}/llvm-nm.txt"
    "${llvm_tools_dir}/llvm-objdump" -h "${KERNEL_ELF}" > "${REPORT_DIR}/sections.txt"
    sha256sum "${KERNEL_ELF}" > "${REPORT_DIR}/dali-kernel.sha256"
    printf 'target=%s\nfeatures=%s\nelf=%s\nmap=%s\n' \
        "${EMBEDDED_TARGET}" "${PRODUCTION_FEATURES}" "${KERNEL_ELF}" "${MAP_FILE}" \
        > "${REPORT_DIR}/build-metadata.txt"
}

main() {
    require_command cargo
    require_command rustc
    require_command sha256sum
    cd -- "${REPOSITORY_ROOT}"
    build_kernel
    local llvm_tools_dir
    llvm_tools_dir="$(find_llvm_tools)"
    write_report "${llvm_tools_dir}"
    for report in "${REPORT_DIR}/llvm-size.txt" "${REPORT_DIR}/llvm-nm.txt" \
        "${REPORT_DIR}/sections.txt" "${REPORT_DIR}/dali-f405-memory.map" \
        "${REPORT_DIR}/dali-kernel.sha256" "${REPORT_DIR}/build-metadata.txt"; do
        [[ -s "${report}" ]] || {
            printf 'ERROR: report artifact is empty: %s\n' "${report}" >&2
            exit 1
        }
    done
    printf 'Memory report written to %s\n' "${REPORT_DIR}"
}

main "$@"
