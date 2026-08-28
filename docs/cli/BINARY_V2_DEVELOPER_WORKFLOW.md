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
  -> Targets cartridge record
  -> AMRN signature
```

Never use an application key as a bundle key unless the signed root policy
explicitly authorizes that same key for both roles. Never print or commit a
private seed. Seeds must not be copied to the SD card or included in a bundle.

## 2. Repository bootstrap and trust ownership

The repository bootstrap workflow is implemented by three host-side commands.
They create and update signed Binary v2 metadata; they do not move private keys
to the SD card and they do not replace the trust owner's key-custody policy.

`repository init` creates a new repository with a root policy, role metadata,
an empty Targets document, an empty Revocation document, and the cartridges and
delegations directories. The root signing seed is used for the initial role
documents. The bundle seed is separately authorized by the generated `bundle`
role.

Run this only for a new repository. It refuses to overwrite an existing path:

```bash
"$DALI_CLI" metadata repository init \
  --output "$DALI_REPO" \
  --root-signing-key "$DALI_KEYS/root.seed" \
  --root-key-id "$DALI_ROOT_KEY_ID" \
  --bundle-signing-key "$DALI_KEYS/bundle.seed" \
  --bundle-key-id "$DALI_BUNDLE_KEY_ID"
```

The repository then contains:

```text
<repository>/metadata/root.dmb
<repository>/metadata/timestamp.dmb
<repository>/metadata/snapshot.dmb
<repository>/metadata/targets.dmb
<repository>/metadata/revocations.dmb
<repository>/metadata/delegat/
<repository>/cartridges/
```

`repository add-developer` adds a signed delegation file, adds its delegation
identifier to `targets.dmb`, and refreshes the signed snapshot and timestamp.
It requires the root seed because the bootstrap policy uses the root signer for
the Delegation, Targets, Snapshot, and Timestamp roles:

```bash
"$DALI_CLI" metadata repository add-developer \
  --input "$DALI_REPO" \
  --signing-key "$DALI_KEYS/root.seed" \
  --developer-id "developer-one" \
  --developer-key-id "$DALI_DEVELOPER_KEY_ID" \
  --developer-public-key "$DALI_DEVELOPER_PUBLIC_KEY" \
  --delegation-id "developer-one-delegation" \
  --namespace "developer-one" \
  --target "f405" \
  --abi 3
```

`DALI_DEVELOPER_PUBLIC_KEY` is the public key from the developer anchor file;
the private developer seed remains local and is used only to sign the AMRN.

`repository publish` re-signs the current role documents and creates the
signed `bundle.manifest`. The repository must contain at least one `.amrn`
cartridge and one delegation before publication can produce a valid bundle:

```bash
"$DALI_CLI" metadata repository publish \
  --input "$DALI_REPO" \
  --root-signing-key "$DALI_KEYS/root.seed" \
  --bundle-signing-key "$DALI_KEYS/bundle.seed" \
  --bundle-key-id "$DALI_BUNDLE_KEY_ID" \
  --target-profile f405 \
  --version 1
```

The cartridge must be copied to `cartridges/<sha256>.amrn` before `publish`.
Cartridge authorization records in `targets.dmb` are a separate required
operation for kernel execution and are created by `register-cartridge` below.
Do not claim a hardware-ready repository until that record references the
developer, delegation, target profile, ABI, slot, digest, and cartridge ID.

For a repository supplied by an external trust owner, the same required files
must already exist. If `metadata/root.dmb` is absent, do not run inspection or
verification; first obtain the signed chain or bootstrap it with `init`.

## 3. Prepare the host

Run these commands from the repository root:

```bash
cd /path/to/dali-kernel

export DALI_ROOT=/path/to/dali-kernel
export DALI_CLI="$DALI_ROOT/target/debug/dali"
export DALI_KEYS=/secure/path/dali/keys
export DALI_REPO=/tmp/dali-repository
export DALI_MOUNT=/run/media/user/DALI

mkdir -p "$DALI_KEYS"
chmod 700 "$DALI_KEYS"
umask 077

cargo build -p dali-cli
```

Generate the repository trust-owner keys before initializing a new repository:

```bash
"$DALI_CLI" key generate \
  --private-output "$DALI_KEYS/root.seed" \
  --public-output "$DALI_KEYS/root-anchor.toml"

"$DALI_CLI" key generate \
  --private-output "$DALI_KEYS/bundle.seed" \
  --public-output "$DALI_KEYS/bundle-anchor.toml"

export DALI_ROOT_KEY_ID="$(sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' "$DALI_KEYS/root-anchor.toml")"
export DALI_BUNDLE_KEY_ID="$(sed -n 's/.*key_id = "\([^"]*\)".*/\1/p' "$DALI_KEYS/bundle-anchor.toml")"
```

Inspect only the public fragments. Never print either seed:

```bash
cat "$DALI_KEYS/root-anchor.toml"
cat "$DALI_KEYS/bundle-anchor.toml"
chmod 600 "$DALI_KEYS/root.seed" "$DALI_KEYS/bundle.seed"
```

## 4. Generate a developer/application key

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

## 5. Initialize and authorize the developer key

For a new repository, initialize the signed role chain before adding a
developer:

```bash
"$DALI_CLI" metadata repository init \
  --output "$DALI_REPO" \
  --root-signing-key "$DALI_KEYS/root.seed" \
  --root-key-id "$DALI_ROOT_KEY_ID" \
  --bundle-signing-key "$DALI_KEYS/bundle.seed" \
  --bundle-key-id "$DALI_BUNDLE_KEY_ID"
```

The new public key must be present in signed delegation and Targets metadata.
Updating `dali.toml` alone is not authorization.

Now add the developer's public key to a signed delegation and Targets policy:

```bash
find "$DALI_REPO" -maxdepth 3 -type f -print | sort
rg -n "$DALI_DEVELOPER_KEY_ID|developer|delegation|targets" "$DALI_REPO"
```

For an externally supplied repository, skip `init` and inspect the files only
after the trust owner has supplied the signed chain. The optional host checks
are:

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

## 6. Confirm bundle-key authorization

The bundle key was generated before repository initialization. Inspect the
public fragment and confirm its ID is present in the signed root policy:

```bash
cat "$DALI_KEYS/bundle-anchor.toml"

echo "$DALI_BUNDLE_KEY_ID"
```

The extracted ID must be authorized in the signed root `bundle` role. The
shell `export` command does not perform authorization:

```bash
rg -n "$DALI_BUNDLE_KEY_ID|bundle" "$DALI_REPO"
```

If the ID is absent from the root policy, stop. A signed root update is
required before bundle generation can succeed.

## 7. Update the application manifest

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
metadata for this cartridge.

## 8. Build and inspect the AMRN cartridge

```bash
cd "$DALI_ROOT/apps/dali-app-relocation-fixture"

export DALI_SIGNING_KEY_HEX="$(
  tr -d '[:space:]' < "$DALI_KEYS/developer.seed"
)"

"$DALI_CLI" app build

export APP_CARTRIDGE="$DALI_ROOT/apps/dali-app-relocation-fixture/target/thumbv7em-none-eabihf/release/dali-app-relocation-fixture.amrn"

test -f "$APP_CARTRIDGE"
"$DALI_CLI" inspect --input "$APP_CARTRIDGE"
```

The reported AMRN `signing_key_id` must equal `DALI_DEVELOPER_KEY_ID`.

## 9. Prepare the content-addressed cartridge repository

Do not delete an existing repository without reviewing it first:

```bash
if [ -e "$DALI_REPO" ]; then
  echo "Repository already exists: $DALI_REPO"
  echo "Review it before continuing; do not delete it automatically."
else
  mkdir -p "$DALI_REPO/cartridges"
fi

export CARTRIDGE_DIGEST="$(
  sha256sum "$APP_CARTRIDGE" | awk '{print tolower($1)}'
)"

cp "$APP_CARTRIDGE" "$DALI_REPO/cartridges/${CARTRIDGE_DIGEST}.amrn"
test -f "$DALI_REPO/cartridges/${CARTRIDGE_DIGEST}.amrn"
echo "$CARTRIDGE_DIGEST"
```

The signed Targets record is created by `repository register-cartridge` and must
contain this exact digest, cartridge ID, target profile, slot, and developer
delegation reference. Run the registration command below after this copy step.

Register the cartridge. The command reads the cartridge identity, target profile,
ABI, slot, and developer key ID from the manifest, validates them against the
AMRN, and derives the cartridge digest from the actual file. The delegation ID
and namespace are explicit because they are repository policy choices, not AMRN
fields:

```bash
"$DALI_CLI" metadata repository register-cartridge \
  --input "$DALI_REPO" \
  --cartridge "$APP_CARTRIDGE" \
  --manifest "$DALI_ROOT/apps/dali-app-relocation-fixture/dali.toml" \
  --delegation-id "developer-one-delegation" \
  --namespace "developer-one" \
  --signing-key "$DALI_KEYS/root.seed"
```

## 10. Generate and verify the Binary v2 bundle

This command re-signs the current metadata chain and creates the signed bundle
manifest. It does not create or rotate the root trust policy:

```bash
"$DALI_CLI" metadata repository publish \
  --input "$DALI_REPO" \
  --root-signing-key "$DALI_KEYS/root.seed" \
  --bundle-signing-key "$DALI_KEYS/bundle.seed" \
  --bundle-key-id "$DALI_BUNDLE_KEY_ID" \
  --target-profile f405 \
  --version 1
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

## 11. Copy the repository to the SD card

Confirm the mount before writing:

```bash
findmnt -rn -t vfat -o TARGET,SOURCE
echo "$DALI_MOUNT"
```

Copy only after confirming that the target is the Dali SD card:

```bash
sudo cp -a "$DALI_REPO/metadata" "$DALI_MOUNT/"
sudo cp -a "$DALI_REPO/cartridges" "$DALI_MOUNT/"
sudo cp "$DALI_REPO/bundle.manifest" "$DALI_MOUNT/"
sync

find "$DALI_MOUNT" -maxdepth 3 -type f \
  \( -name '*.dmb' -o -name '*.amrn' -o -name 'bundle.manifest' \) \
  -print | sort
```

## 12. Build and flash the release kernel

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
"$DALI_CLI" device console --port "$DALI_CONSOLE_PORT"
```

Use the available CDC port reported by `dali device list` if it is not
the selected CDC console port.

In terminal 2, flash the kernel:

```bash
pkill -TERM -f '^probe-rs ' || true

sudo "$DALI_CLI" device flash f405 \
  --transport probe \
  --input "$DALI_ROOT/target/thumbv7em-none-eabihf/release/dali-kernel"
```

## 13. Expected hardware evidence

For one valid cartridge, the console should include:

```text
[INFO][BOOT] [STORAGE] SDIO card initialized
[INFO][BOOT] [STORAGE] Read block 0 successfully
[INFO][BOOT] [LOADER] AMRN header and payload validated
[INFO][SECURITY] [SECURITY] AMRN signature verified
[INFO][BOOT] [LOADER] Loaded 1 application cartridge(s) into declared slots
[INFO][SECURITY] [SECURITY] Application lifecycle: Ready
[INFO][SECURITY] [SECURITY] Active application context: Running
```

Multi-cartridge acceptance requires two real signed AMRN cartridges, two matching
Targets records, and distinct valid slots. Copying one cartridge twice is not a
multi-cartridge test. With two valid records, the expected loader line is:

```text
[INFO][BOOT] [LOADER] Loaded 2 application cartridge(s) into declared slots
```

## 14. Error interpretation

- `unknown key`: the key is absent from the trust policy;
- `unauthorized`: the key exists but lacks permission for the role or target;
- `revoked key`: the key is listed in revocation metadata;
- `invalid signature`: the seed, key ID, or signed bytes do not match;
- `invalid hash`: the cartridge differs from the Targets digest;
- `UnsupportedFormatVersion`: the SD card contains an incompatible cartridge;
- `Loaded 1 application cartridge(s)`: one matching executable was discovered;
  this is not an error.

Compilation and host verification do not prove hardware acceptance. Record the
board, wiring, SD card/filesystem, firmware build, console port, expected logs,
observed logs, and result for every physical run.
