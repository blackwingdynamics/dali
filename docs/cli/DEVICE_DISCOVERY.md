# Dali Device Discovery Contract

## Scope

Device discovery is a host-side, read-only operation. It observes devices
already visible to the operating system and reports normalized records to the
Dali CLI. Discovery must not reset, flash, attach, open a terminal, mount
storage, or change device state.

The contract is transport-neutral. Transport adapters own platform-specific
enumeration and parsing; the CLI owns filtering, normalization, ordering, and
presentation.

The normalized record model is implemented in the hardware-neutral
`dali-device` crate. The crate does not enumerate host devices and has no
transport or board dependency.

## Discovery transports

The initial transport set is:

| Transport | Host-visible source | State observed |
| --- | --- | --- |
| `probe` | SWD debug-probe inventory | probe identity and optional chip information |
| `dfu` | USB device inventory | bootloader identity and DFU availability |
| `cdc` | USB serial inventory | runtime console identity and port path |
| `bulk` | Dali USB installer interface | cartridge installation endpoint |

The transport names are stable CLI vocabulary. A transport adapter may report
that a device is present but partially identified; discovery must not infer a
target from a missing field.

## Normalized device record

Each discovered record contains these fields:

| Field | Meaning | Required |
| --- | --- | --- |
| `id` | Stable identifier supplied by the transport adapter | yes |
| `transport` | One of the supported transport values | yes |
| `target` | Matching target profile name from the generated registry | no |
| `vendor` | Transport-reported vendor identifier | no |
| `product` | Transport-reported product or probe name | no |
| `serial` | Transport-reported serial identifier | no |
| `path` | Host path when the transport exposes one | no |
| `state` | Normalized availability state | yes |
| `capabilities` | Operations supported by the observed device | yes |
| `diagnostic` | Bounded non-secret discovery detail | no |

Transport-specific identifiers remain in the adapter. The normalized record
must not embed board names, USB identifiers, chip names, paths, or capabilities
as literals in command logic. Target metadata comes from `targets/*.toml` and
the generated registry.

## States

The initial state vocabulary is:

- `available`: the device is visible and its transport can use it;
- `identified`: the device is visible and matched to a target profile;
- `unidentified`: the device is visible but target matching is incomplete;
- `in_use`: the host reports that another process owns the device;
- `error`: enumeration returned a bounded diagnostic error.

Discovery reports every record it can obtain. One adapter error must not hide
records from other transports, but the command must return a non-zero result
when an adapter reports an operational discovery error.

## Identity and deduplication

The adapter supplies the primary stable `id`. The CLI must not construct an
identity from a path alone because paths can change after reconnects. When the
same physical device is visible through multiple transports, records remain
separate because each transport represents a different operation boundary.

Runtime CDC devices may not expose a serial number. In that case the Linux
adapter may report a connection-scoped identity derived from udev's physical
topology property, but it must mark the record `unidentified` and must not use
that identity for target matching or unattended destructive operations.

Within one transport, duplicate records with the same adapter-provided `id`
are collapsed. Output ordering is deterministic: transport order follows the
declared discovery transport registry, then records are ordered by `id`.

## Matching and filtering

Target matching uses the generated target registry. A DFU record is
`identified` when its vendor/product pair matches the optional `[dfu]` identity
in one declared profile. Probe and runtime matching use their own declared
transport identities. Unknown or conflicting identities remain
`unidentified`; the CLI must not guess or silently select a profile.

Future filters may select transport, target, or identifier, but filtering must
occur after discovery and must not alter adapter behavior.

## Output contract

The initial `dali device list` presentation is human-readable and stable in
field names. It reports probe, DFU, Linux CDC, and Dali bulk-installer records.
The bulk record is identified by the target manifest's USB vendor/product pair
and the vendor-specific installer interface. CDC records without a declared
serial are marked `unidentified` and use a connection-scoped udev topology
identity. The output must not include secrets or unrestricted kernel/USB dumps.
A future structured output mode may expose the normalized record fields, but it
must be explicitly versioned before scripts depend on it.

## Linux USB permission setup

The installer claim check opens the USB device node. On Linux, add a udev rule
once so the active desktop user receives access to the Dali F405 USB identity:

~~~bash
sudo tee /etc/udev/rules.d/99-dali-usb.rules >/dev/null <<'EOF'
SUBSYSTEM=="usb", ATTR{idVendor}=="1209", ATTR{idProduct}=="da11", TAG+="uaccess"
EOF
sudo udevadm control --reload-rules
sudo udevadm trigger
~~~

The closing EOF marker must start in column one with no preceding spaces.
Disconnect and reconnect the board after reloading the rules. Verify discovery
first:

~~~text
cargo run --quiet -p dali-cli --bin dali -- device list
~~~

Then verify that the installer interface can be claimed without sending
payload data:

~~~text
cargo run --quiet -p dali-cli --bin dali -- device info usb:1209:DA11:no-serial
~~~

The command must print installer interface: verified. This check does not
write Flash or transfer a cartridge.

An empty result is successful and distinct from an adapter error. Hardware
commands must not be invoked as a side effect of an empty result.

## Failure and safety boundary

Adapters must bound host reads and convert platform errors into typed discovery
failures. The CLI reports the transport and diagnostic category without
leaking credentials, private paths beyond the declared device path, or raw
memory data.

This contract does not prove that a device is powered correctly, that firmware
is valid, that SWD access will succeed, that DFU flashing will work, or that a
CDC terminal can receive logs. Those are separate device-operation and
hardware-evidence boundaries.
