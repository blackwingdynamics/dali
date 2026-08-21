# Binary v2 Developer and Hardware Acceptance Workflow

This document is the end-to-end workflow for a new Dali application developer.
It covers key generation, authorization prerequisites, AMRN signing, Binary v2
repository preparation, SD-card installation, kernel flashing, and F405 log
verification.

## 1. Key ownership and trust roles

Dali uses separate Ed25519 keys for separate trust roles:

- **Developer/application key** signs one `.amrn` application. Its `key_id` is
  stored in `dali.toml` and the AMRN header.
- **Bundle key** signs the repository bundle manifest. Its `key_id` must be
  authorized by the root metadata `bundle` role.

Generating a key does not authorize it. The trust chain is:

```text
Root policy
  -> Bundle role / bundle key
  -> Delegation metadata / developer key
  -> Targets package record
  -> AMRN signature
```

Never use an application key as a bundle key unless the signed root policy
explicitly authorizes that same key for both roles. Never print or commit a
private seed. Seeds must not be copied to the SD card or included in a bundle.

## 2. Prepare the host

Run these commands from the repository root:

```bash
cd /home/magradze/Projects/rust/dali-kernel

export DALI_ROOT=/home/magradze/Projects/rust/dali-kernel
export DALI_CLI="$DALI_ROOT/target/debug/dali"
export DALI_KEYS=/home/magradze/.config/dali/keys
export DALI_REPO=/tmp/dali-repository
export DALI_MOUNT=/run/media/magradze/DALI

mkdir -p "$DALI_KEYS"
chmod 700 "$DALI_KEYS"
umask 077

cargo build -p dali-cli
```

## 3. Generate a developer/application key

```bash
"$DALI_CLI" key generate \
  --private-output "$DALI_KEYS/developer.seed" \
  --public-output "$DALI_KEYS/developer-anchor.toml"
```

The public fragment may be inspected:

```bash
cat "$DALI_KEYS/developer-anchor.toml"
```

Extract its public key identifier without printing the seed:

```bash
export DALI_DEVELOPER_KEY_ID="$(
  sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' \
  "$DALI_KEYS/developer-anchor.toml"
)"

echo "$DALI_DEVELOPER_KEY_ID"
chmod 600 "$DALI_KEYS/developer.seed"
```

## 4. Authorize the developer key

The new public key must be present in signed delegation and Targets metadata.
Updating `dali.toml` alone is not authorization.

Inspect the prepared repository:

```bash
find "$DALI_REPO" -maxdepth 3 -type f -print | sort
rg -n "$DALI_DEVELOPER_KEY_ID|developer|delegation|targets" "$DALI_REPO"
```

Then run the host checks:

```bash
"$DALI_CLI" metadata bundle inspect \
  --input "$DALI_REPO" \
  --metadata-format binary-v2

"$DALI_CLI" metadata bundle verify \
  --input "$DALI_REPO" \
  --metadata-format binary-v2
```

The current CLI generates and verifies bundle manifests, but does not provide
an automatic root-policy rotation command. If the new developer key is not in
the signed root/delegation policy, stop and obtain a signed metadata update
from the repository trust owner. Do not edit binary root metadata manually and
do not continue to hardware flashing with an unauthorized key.

## 5. Generate and authorize a bundle key

Generate a separate key:

```bash
"$DALI_CLI" key generate \
  --private-output "$DALI_KEYS/bundle.seed" \
  --public-output "$DALI_KEYS/bundle-anchor.toml"
```

Inspect the public fragment and extract the ID:

```bash
cat "$DALI_KEYS/bundle-anchor.toml"

export DALI_BUNDLE_KEY_ID="$(
  sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' \
  "$DALI_KEYS/bundle-anchor.toml"
)"

echo "$DALI_BUNDLE_KEY_ID"
chmod 600 "$DALI_KEYS/bundle.seed"
```

The extracted ID must be authorized in the signed root `bundle` role. The
shell `export` command does not perform authorization:

```bash
rg -n "$DALI_BUNDLE_KEY_ID|bundle" "$DALI_REPO"
```

If the ID is absent from the root policy, stop. A signed root update is
required before bundle generation can succeed.

## 6. Update the application manifest

Open the application manifest:

```bash
nano "$DALI_ROOT/apps/dali-app-relocation-fixture/dali.toml"
```

Set the application signing key to the generated developer key:

```toml
signing_key_id = "<DALI_DEVELOPER_KEY_ID>"
```

Replace the placeholder with:

```bash
echo "$DALI_DEVELOPER_KEY_ID"
```

The same ID must be authorized by delegation and referenced by the Targets
metadata for this package.

## 7. Build and inspect the AMRN package

```bash
cd "$DALI_ROOT/apps/dali-app-relocation-fixture"

export DALI_SIGNING_KEY_HEX="$(
  tr -d '[:space:]' < "$DALI_KEYS/developer.seed"
)"

"$DALI_CLI" app build

export APP_PACKAGE="$DALI_ROOT/apps/dali-app-relocation-fixture/target/thumbv7em-none-eabihf/release/dali-app-relocation-fixture.amrn"

test -f "$APP_PACKAGE"
"$DALI_CLI" inspect --input "$APP_PACKAGE"
```

The reported AMRN `signing_key_id` must equal `DALI_DEVELOPER_KEY_ID`.

## 8. Prepare the content-addressed package repository

Do not delete an existing repository without reviewing it first:

```bash
if [ -e "$DALI_REPO" ]; then
  echo "Repository already exists: $DALI_REPO"
  exit 1
fi

mkdir -p "$DALI_REPO/packages"

export PACKAGE_DIGEST="$(
  sha256sum "$APP_PACKAGE" | awk '{print tolower($1)}'
)"

cp "$APP_PACKAGE" "$DALI_REPO/packages/${PACKAGE_DIGEST}.amrn"
test -f "$DALI_REPO/packages/${PACKAGE_DIGEST}.amrn"
echo "$PACKAGE_DIGEST"
```

The signed Targets record must contain this exact digest, package ID, target
profile, slot, and developer delegation reference.

## 9. Generate and verify the Binary v2 bundle

This command creates the signed bundle manifest. It does not create the root
trust policy:

```bash
"$DALI_CLI" metadata bundle generate \
  --input "$DALI_REPO" \
  --output "$DALI_REPO/bundle.manifest" \
  --target-profile f405 \
  --version 1 \
  --signing-key "$DALI_KEYS/bundle.seed" \
  --signer-key-id "$DALI_BUNDLE_KEY_ID" \
  --metadata-format binary-v2
```

Inspect and verify it:

```bash
"$DALI_CLI" metadata bundle inspect \
  --input "$DALI_REPO" \
  --metadata-format binary-v2

"$DALI_CLI" metadata bundle verify \
  --input "$DALI_REPO" \
  --metadata-format binary-v2
```

Stop on `unknown key`, `unauthorized`, `revoked key`, `invalid chain`,
`invalid signature`, or `invalid hash`. Do not flash a repository that fails
host verification.

## 10. Copy the repository to the SD card

Confirm the mount before writing:

```bash
findmnt -rn -t vfat -o TARGET,SOURCE
echo "$DALI_MOUNT"
```

Copy only after confirming that the target is the Dali SD card:

```bash
sudo cp -a "$DALI_REPO/metadata" "$DALI_MOUNT/"
sudo cp -a "$DALI_REPO/packages" "$DALI_MOUNT/"
sudo cp "$DALI_REPO/bundle.manifest" "$DALI_MOUNT/"
sync

find "$DALI_MOUNT" -maxdepth 3 -type f \
  \( -name '*.dmb' -o -name '*.amrn' -o -name 'bundle.manifest' \) \
  -print | sort
```

## 11. Build and flash the release kernel

```bash
cd "$DALI_ROOT"

cargo build -p dali-kernel \
  --release \
  --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-relocation,repository-loader,storage-write \
  --target thumbv7em-none-eabihf
```

In terminal 1, start the console:

```bash
pkill -TERM -f '^picocom ' || true
"$DALI_CLI" device list
"$DALI_CLI" device console --port /dev/ttyACM0
```

Use the available CDC port reported by `dali device list` if it is not
`/dev/ttyACM0`.

In terminal 2, flash the kernel:

```bash
pkill -TERM -f '^probe-rs ' || true

sudo "$DALI_CLI" device flash f405 \
  --transport probe \
  --input "$DALI_ROOT/target/thumbv7em-none-eabihf/release/dali-kernel"
```

## 12. Expected hardware evidence

For one valid package, the console should include:

```text
[INFO][BOOT] [STORAGE] SDIO card initialized
[INFO][BOOT] [STORAGE] Read block 0 successfully
[INFO][BOOT] [LOADER] AMRN header and payload validated
[INFO][SECURITY] [SECURITY] AMRN signature verified
[INFO][BOOT] [LOADER] Loaded 1 application package(s) into declared slots
[INFO][SECURITY] [SECURITY] Application lifecycle: Ready
[INFO][SECURITY] [SECURITY] Active application context: Running
```

Multi-package acceptance requires two real signed AMRN packages, two matching
Targets records, and distinct valid slots. Copying one package twice is not a
multi-package test. With two valid records, the expected loader line is:

```text
[INFO][BOOT] [LOADER] Loaded 2 application package(s) into declared slots
```

## 13. Error interpretation

- `unknown key`: the key is absent from the trust policy;
- `unauthorized`: the key exists but lacks permission for the role or target;
- `revoked key`: the key is listed in revocation metadata;
- `invalid signature`: the seed, key ID, or signed bytes do not match;
- `invalid hash`: the package differs from the Targets digest;
- `UnsupportedFormatVersion`: the SD card contains an incompatible package;
- `Loaded 1 application package(s)`: one matching executable was discovered;
  this is not an error.

Compilation and host verification do not prove hardware acceptance. Record the
board, wiring, SD card/filesystem, firmware build, console port, expected logs,
observed logs, and result for every physical run.
