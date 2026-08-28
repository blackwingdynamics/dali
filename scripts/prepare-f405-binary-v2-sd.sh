#!/usr/bin/env bash

set -Eeuo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly DALI_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly APP_DIR="${DALI_ROOT}/apps/dali-app-relocation-fixture"
readonly APP_MANIFEST="${APP_DIR}/dali.toml"
readonly APP_CARTRIDGE="${APP_DIR}/target/thumbv7em-none-eabihf/release/dali-app-relocation-fixture.amrn"
readonly TARGET_MANIFEST="${DALI_ROOT}/targets/f405.toml"

readonly RUN_ID="${DALI_RUN_ID:-$(date +%Y%m%d-%H%M%S)}"
readonly DALI_CLI="${DALI_CLI:-${DALI_ROOT}/target/debug/dali}"
readonly DALI_MOUNT="${DALI_MOUNT:-/run/media/${USER}/DALI}"
readonly DALI_KEYS="${DALI_KEYS:-${XDG_CONFIG_HOME:-${HOME}/.config}/dali/f405-acceptance-${RUN_ID}}"
readonly DALI_REPO="${DALI_REPO:-/tmp/dali-repository-f405-${RUN_ID}}"
readonly DALI_ROOT_SEED="${DALI_ROOT_SEED:-${XDG_CONFIG_HOME:-${HOME}/.config}/dali/keys/f405-release.seed}"

die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "required command is missing: $1"
}

require_file() {
    [[ -f "$1" ]] || die "required file does not exist: $1"
}

restore_manifest() {
    if [[ -n "${MANIFEST_BACKUP:-}" && -f "${MANIFEST_BACKUP}" ]]; then
        cp -- "${MANIFEST_BACKUP}" "${APP_MANIFEST}"
        rm -f -- "${MANIFEST_BACKUP}"
    fi
}

cleanup_on_error() {
    restore_manifest
}

trap cleanup_on_error EXIT

require_command cargo
require_command find
require_command findmnt
require_command sha256sum
require_command awk
require_command grep
require_command sed
require_command sudo
require_command tr
require_command mktemp
require_file "${APP_MANIFEST}"
require_file "${TARGET_MANIFEST}"
case "${DALI_ROOT_SEED}" in
    /tmp/*|/var/tmp/*)
        die "DALI_ROOT_SEED must be persistent protected storage, not a temporary path: ${DALI_ROOT_SEED}"
        ;;
esac
require_file "${DALI_ROOT_SEED}"

if [[ ! -d "${DALI_MOUNT}" ]]; then
    die "SD mount directory does not exist: ${DALI_MOUNT}"
fi

readonly MOUNT_INFO="$(findmnt -rn -T "${DALI_MOUNT}" -o TARGET,SOURCE,FSTYPE 2>/dev/null || true)"
[[ -n "${MOUNT_INFO}" ]] || die "${DALI_MOUNT} is not a mounted filesystem"

if [[ -e "${DALI_REPO}" ]]; then
    readonly TEMP_REPOSITORY_SUFFIX="${DALI_REPO#/tmp/dali-repository-f405-}"
    if [[ "${DALI_REPO}" != /tmp/dali-repository-f405-* \
        || -z "${TEMP_REPOSITORY_SUFFIX}" \
        || "${TEMP_REPOSITORY_SUFFIX}" == */* ]]; then
        die "repository path already exists and is not an automatically managed temporary path: ${DALI_REPO}"
    fi
    printf 'Removing previous temporary repository: %s\n' "${DALI_REPO}"
    rm -rf -- "${DALI_REPO}"
fi

if [[ -e "${DALI_KEYS}" ]]; then
    readonly TEMP_KEYS_SUFFIX="${DALI_KEYS##*/f405-acceptance-}"
    if [[ "${DALI_KEYS}" != */.config/dali/f405-acceptance-* \
        || -z "${TEMP_KEYS_SUFFIX}" \
        || "${TEMP_KEYS_SUFFIX}" == */* ]]; then
        die "key path already exists and is not an automatically managed temporary path: ${DALI_KEYS}"
    fi
    printf 'Removing previous temporary key directory: %s\n' "${DALI_KEYS}"
    rm -rf -- "${DALI_KEYS}"
fi

umask 077
mkdir -p -- "${DALI_KEYS}"
chmod 700 -- "${DALI_KEYS}"
mkdir -p -- "$(dirname -- "${DALI_REPO}")"

printf '[1/9] Building dali-cli\n'
cargo build --manifest-path "${DALI_ROOT}/Cargo.toml" -p dali-cli
require_file "${DALI_CLI}"

printf '[2/9] Loading the provisioned root and generating bundle/developer keys\n'
"${DALI_CLI}" key generate \
    --private-output "${DALI_KEYS}/bundle.seed" \
    --public-output "${DALI_KEYS}/bundle-anchor.toml"
"${DALI_CLI}" key generate \
    --private-output "${DALI_KEYS}/developer.seed" \
    --public-output "${DALI_KEYS}/developer-anchor.toml"

readonly DALI_ROOT_KEY_ID="$(sed -n '/release_trust_anchors = \[/,/]/s/.*key_id = "\([^"]*\)".*/\1/p' "${TARGET_MANIFEST}")"
readonly DALI_BUNDLE_KEY_ID="$(sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' "${DALI_KEYS}/bundle-anchor.toml")"
readonly DALI_DEVELOPER_KEY_ID="$(sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' "${DALI_KEYS}/developer-anchor.toml")"
readonly DALI_DEVELOPER_PUBLIC_KEY="$(sed -n 's/.*public_key = "\([^"]*\)".*/\1/p' "${DALI_KEYS}/developer-anchor.toml")"

[[ -n "${DALI_ROOT_KEY_ID}" ]] || die "release root key ID was not found in ${TARGET_MANIFEST}"
[[ -n "${DALI_BUNDLE_KEY_ID}" ]] || die "bundle key ID was not extracted"
[[ -n "${DALI_DEVELOPER_KEY_ID}" ]] || die "developer key ID was not extracted"
[[ -n "${DALI_DEVELOPER_PUBLIC_KEY}" ]] || die "developer public key was not extracted"

chmod 600 -- "${DALI_KEYS}"/*.seed

printf '[3/9] Updating the temporary application signing identity\n'
MANIFEST_BACKUP="$(mktemp "${DALI_ROOT}/.dali-manifest-backup.XXXXXX")"
cp -- "${APP_MANIFEST}" "${MANIFEST_BACKUP}"
grep -q '^signing_key_id = ' "${APP_MANIFEST}" \
    || die "${APP_MANIFEST} has no signing_key_id field"
sed -i -E "s/^signing_key_id = .*/signing_key_id = \"${DALI_DEVELOPER_KEY_ID}\"/" "${APP_MANIFEST}"

printf '[4/9] Building and inspecting the signed AMRN cartridge\n'
export DALI_SIGNING_KEY_HEX="$(tr -d '[:space:]' < "${DALI_KEYS}/developer.seed")"
(cd -- "${APP_DIR}" && "${DALI_CLI}" app build)
require_file "${APP_CARTRIDGE}"
"${DALI_CLI}" inspect --input "${APP_CARTRIDGE}"

printf '[5/9] Initializing the Binary v2 repository\n'
"${DALI_CLI}" metadata repository init \
    --output "${DALI_REPO}" \
    --root-signing-key "${DALI_ROOT_SEED}" \
    --root-key-id "${DALI_ROOT_KEY_ID}" \
    --bundle-signing-key "${DALI_KEYS}/bundle.seed" \
    --bundle-key-id "${DALI_BUNDLE_KEY_ID}"

printf '[6/9] Authorizing the developer and registering the cartridge\n'
"${DALI_CLI}" metadata repository add-developer \
    --input "${DALI_REPO}" \
    --signing-key "${DALI_ROOT_SEED}" \
    --developer-id developer-one \
    --developer-key-id "${DALI_DEVELOPER_KEY_ID}" \
    --developer-public-key "${DALI_DEVELOPER_PUBLIC_KEY}" \
    --delegation-id developer-one-delegation \
    --namespace developer-one \
    --target f405 \
    --abi 3

readonly CARTRIDGE_DIGEST="$(sha256sum "${APP_CARTRIDGE}" | awk '{print tolower($1)}')"
cp -- "${APP_CARTRIDGE}" "${DALI_REPO}/cartridges/${CARTRIDGE_DIGEST}.amrn"
require_file "${DALI_REPO}/cartridges/${CARTRIDGE_DIGEST}.amrn"

"${DALI_CLI}" metadata repository register-cartridge \
    --input "${DALI_REPO}" \
    --cartridge "${APP_CARTRIDGE}" \
    --manifest "${APP_MANIFEST}" \
    --delegation-id developer-one-delegation \
    --namespace developer-one \
    --signing-key "${DALI_ROOT_SEED}"

printf '[7/9] Publishing and verifying the complete host bundle\n'
"${DALI_CLI}" metadata repository publish \
    --input "${DALI_REPO}" \
    --root-signing-key "${DALI_ROOT_SEED}" \
    --bundle-signing-key "${DALI_KEYS}/bundle.seed" \
    --bundle-key-id "${DALI_BUNDLE_KEY_ID}" \
    --target-profile f405 \
    --version 1
"${DALI_CLI}" metadata bundle inspect \
    --input "${DALI_REPO}" \
    --metadata-format binary-v2
"${DALI_CLI}" metadata bundle verify \
    --input "${DALI_REPO}" \
    --metadata-format binary-v2

printf '[8/9] Confirming the selected SD card\n'
printf 'Mounted filesystem: %s\n' "${MOUNT_INFO}"
printf 'Target mount:      %s\n' "${DALI_MOUNT}"
read -r -p 'Type WRITE to copy the verified repository to this SD card: ' confirmation
[[ "${confirmation}" == WRITE ]] || die 'SD copy cancelled'

printf '[9/9] Copying verified metadata and cartridges to the SD card\n'
sudo -v
sudo mkdir -p -- "${DALI_MOUNT}/metadata" "${DALI_MOUNT}/cartridges"
sudo find "${DALI_MOUNT}/cartridges" -maxdepth 1 -type f -name '*.amrn' -delete
sudo cp -a -- "${DALI_REPO}/metadata/." "${DALI_MOUNT}/metadata/"
sudo cp -a -- "${DALI_REPO}/cartridges/." "${DALI_MOUNT}/cartridges/"
sudo cp -- "${DALI_REPO}/bundle.manifest" "${DALI_MOUNT}/bundle.manifest"
sync

printf '\nSD copy complete. Repository: %s\n' "${DALI_REPO}"
printf 'Cartridge digest: %s\n' "${CARTRIDGE_DIGEST}"
printf 'Before removing the card, unmount it cleanly from the desktop or with udisksctl.\n'
printf 'The generated seeds remain outside the repository at: %s\n' "${DALI_KEYS}"
