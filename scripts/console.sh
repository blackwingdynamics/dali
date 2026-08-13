#!/usr/bin/env bash

set -euo pipefail

readonly REQUESTED_PORT="$1"
readonly BAUD="$2"
readonly RETRY_DELAY="$3"
shopt -s nullglob
waiting_message_shown=false

find_port() {
    if [[ -n "${REQUESTED_PORT}" ]]; then
        printf '%s\n' "${REQUESTED_PORT}"
        return
    fi

    local candidate
    local candidates=(/dev/ttyACM*)
    for candidate in "${candidates[@]}"; do
        if [[ -r "${candidate}" && -w "${candidate}" ]]; then
            printf '%s\n' "${candidate}"
            return
        fi
    done

    if ((${#candidates[@]} > 0)); then
        printf 'Cannot access any available USB console port. Check the uucp group.\n' >&2
        exit 1
    fi
}

while true; do
    PORT="$(find_port)"
    if [[ -z "${PORT}" || ! -e "${PORT}" ]]; then
        if [[ "${waiting_message_shown}" == false ]]; then
            printf 'Waiting for USB CDC console (/dev/ttyACM*)...\n' >&2
            waiting_message_shown=true
        fi
        sleep "${RETRY_DELAY}"
        continue
    fi

    printf 'Connecting to USB CDC console at %s...\n' "${PORT}" >&2

    if [[ ! -r "${PORT}" || ! -w "${PORT}" ]]; then
        printf 'Cannot access %s. Add your user to the device group and start a new session.\n' "${PORT}" >&2
        exit 1
    fi

    if fuser "${PORT}" >/dev/null 2>&1; then
        printf 'Port %s is already in use by another process.\n' "${PORT}" >&2
        exit 1
    fi

    if picocom "${PORT}" -b "${BAUD}"; then
        exit 0
    fi

    sleep "${RETRY_DELAY}"
done
