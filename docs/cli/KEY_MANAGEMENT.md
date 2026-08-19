# Release Trust-Anchor and Signing-Key Management

## Purpose

Dali package signatures provide authenticity only when the kernel has a trusted
public key for the package's `key_id`. The private signing seed creates package
signatures; the public key is provisioned into the target profile and compiled
into the kernel trust store.

The two materials have different lifecycles:

| Material | Purpose | Repository | Required secrecy |
| --- | --- | --- | --- |
| Ed25519 private seed | Signs AMRN v5 packages | Never commit | Secret |
| Ed25519 public key | Verifies package signatures | `targets/*.toml` | Public |
| 16-byte `key_id` | Selects the public key | Package and target manifest | Public |

The key identifier is an opaque random label. It is not a password, a hash
that authenticates the key, or a replacement for the public key. The signature
is verified with the public key associated with that identifier.

## Current security boundary

The F405 development profile contains the RFC8032 test anchor and is enabled
only for `abi-test-fixtures`. The F405 release profile now contains the public
anchor generated for this checkout. This provisions the verification path, but
does not by itself make the key production-grade: secret custody, rotation
procedure, release build approval, and hardware release acceptance are still
required.

Do not put the RFC8032 test seed or any locally generated private seed into
the repository, a target manifest, firmware, an issue, or a chat message.

## Generate a keypair

Generate the pair on an offline or controlled host. The command obtains both
the private seed and the key identifier from the operating system's CSPRNG,
derives the Ed25519 public key, and refuses to overwrite existing files.

Build the CLI if necessary:

```text
cargo build -p dali-cli
```

Create a private-key directory outside the repository with restrictive default
permissions, then generate the pair:

```text
umask 077
mkdir -p /secure/path/dali/keys

target/debug/dali key generate \
  --private-output /secure/path/dali/keys/f405-release.seed \
  --public-output /secure/path/dali/keys/f405-release-anchor.toml
```

The command creates:

- `f405-release.seed`: a 32-byte Ed25519 seed encoded as uppercase hex and
  written with mode `0600` on Unix hosts;
- `f405-release-anchor.toml`: a public manifest fragment containing `key_id`
  and `public_key`.

The command does not print the seed. It prints only the output paths. If either
path already exists, stop and choose a new path; do not delete or overwrite an
existing key as part of rotation.

The generated public fragment has this form:

```toml
{ key_id = "...32 hexadecimal digits...", public_key = "...64 hexadecimal digits..." }
```

The private seed file must be backed up using the project's approved secret
storage. A lost seed cannot be reconstructed from the public key. A copied or
exposed seed must be treated as compromised and replaced.

## Provision a target release profile

Copy only the public fragment into the selected target manifest:

```toml
[authentication]
development = "unsigned"
release = "ed25519"
development_trust_anchors = [
  { key_id = "...", public_key = "..." },
]
release_trust_anchors = [
  { key_id = "...", public_key = "..." },
]
```

For a production release, the `release_trust_anchors` entry must match the
private seed used to sign the package. The private seed itself must never be
added to `targets/f405.toml` or any other tracked file.

Review the public fragment and target diff carefully. The key identifier and
public key are both fixed-width hexadecimal values. Run the normal workspace
checks after changing a target manifest.

## Sign a release application package

The application manifest must select the release profile and AMRN v5:

```toml
[build]
profile = "release"
format_version = 5
signing_key_id = "...the provisioned key_id..."
```

Build the application with the private seed supplied by a secret mechanism:

```text
export DALI_SIGNING_KEY_HEX="$(tr -d '\n' < /secure/path/dali/keys/f405-release.seed)"
dali app build
unset DALI_SIGNING_KEY_HEX
```

In CI, use the CI secret store instead of a checked-out seed file. Do not use
`echo` to display the variable, do not include it in command traces, and do
not place it in a shell history entry. The CLI reads the seed only to create
the DSIG envelope; the seed is not written into the AMRN package.

Inspect the package before deployment:

```text
dali inspect --input target/thumbv7em-none-eabihf/release/<application>.amrn
```

Confirm that the reported `format_version` is `5`, the `signing_key_id` is
the provisioned identifier, and the package metadata is correct. Inspection is
host/package evidence; it does not replace target hardware acceptance.

## Kernel build and acceptance

The F405 signed-loader image must use the release profile because the
feature-complete development link does not fit the board's flash region:

```text
cargo build -p dali-kernel --release \
  --no-default-features \
  --features board-stm32f405-sd,usb-cdc,abi-authentication \
  --target thumbv7em-none-eabihf
```

The production build must not enable `abi-test-fixtures`. On hardware, a valid
package must log signature verification before application loading. A package
with an unknown key identifier, modified signed content, or malformed DSIG
trailer must be rejected before SRAM copy and application entry.

Record the board, kernel commit, target profile, key identifier, package hash,
SD card/filesystem, power source, console channel, expected output, observed
output, and result in `docs/TESTING.md` or the associated acceptance record.

## Rotation and incident response

The current target profile supports multiple release trust anchors, which
allows a controlled rotation:

1. Generate a new keypair and store its private seed separately.
2. Add the new public key and key identifier to `release_trust_anchors`.
3. Build, flash, and hardware-test a kernel that accepts both old and new
   anchors.
4. Sign new packages with the new key and verify the new key identifier.
5. After the migration window, release another kernel that removes the old
   anchor.

If a private seed is exposed, stop signing immediately, preserve the incident
record, provision a replacement anchor through a firmware update, and treat
packages signed by the exposed key as untrusted. The current implementation
does not provide remote revocation, anti-rollback, or automatic trust-store
updates; those remain separate roadmap work.

## Prohibited practices

- Never commit a private seed, CI secret, or generated key directory.
- Never reuse the RFC8032 development seed for production.
- Never generate production keys in an online random-key website.
- Never copy a private seed into `targets/*.toml` or firmware source.
- Never claim Secure Boot solely from CRC32 or host-side package inspection.
- Never deploy a release package until the target contains the corresponding
  public trust anchor and hardware acceptance has been recorded.
