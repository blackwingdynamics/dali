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

Within one transport, duplicate records with the same adapter-provided `id`
are collapsed. Output ordering is deterministic: transport order follows the
declared discovery transport registry, then records are ordered by `id`.

## Matching and filtering

Target matching uses the generated target registry. A record is `identified`
only when the transport-reported chip, bootloader, or runtime identity matches
one declared profile. Unknown or conflicting identities remain
`unidentified`; the CLI must not guess or silently select a profile.

Future filters may select transport, target, or identifier, but filtering must
occur after discovery and must not alter adapter behavior.

## Output contract

The initial `dali device list` presentation is human-readable and stable in
field names. It currently reports probe and DFU records; CDC discovery remains
pending a stable host identity adapter. The output must not include secrets or unrestricted kernel/USB dumps. A
future structured output mode may expose the normalized record fields, but it
must be explicitly versioned before scripts depend on it.

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
