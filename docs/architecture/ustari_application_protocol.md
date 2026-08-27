# Ustari Application Protocol

Status: **Approved future architecture; implementation pending**.

Ustari is Dali OS's canonical native application protocol for commands,
telemetry, diagnostics, and package transfer. The kernel speaks typed binary
Ustari frames; host tools translate those frames into text, tables, or GUI
views. Ustari does not replace a physical transport, the AMRN package format,
the Trust Store, Secure Boot, MPU policy, watchdog, or local actuator safety.

This document records the accepted architecture for the future shell and
telemetry ecosystem. It does not change the current MVP, the existing
thirteen-step roadmap, the ABI, the boot path, the storage layout, or the
package layout. The protocol remains unavailable until its implementation and
acceptance gates are complete.

## Decision

Dali OS will use one canonical application-layer protocol across USB CDC,
UART, Wi-Fi/BLE bridges, LoRa, and ExpressLRS. The protocol core is
transport-agnostic, `no_std`, zero-heap, fixed-capacity, and bounded.

This is a single semantic protocol, not a promise that all transports share
the same packet size or physical framing. Each adapter owns its byte-stream,
MTU, fragmentation, retry, reconnect, and loss behavior. Every adapter must
terminate at the same Ustari decoder, session security, and authorization
policy boundary.

The target does not parse human-oriented CLI text. `dali shell` and GUI tools
are host-side presentation layers that encode user intent as Ustari messages
and render typed responses. A minimal recovery profile may expose only a
strictly bounded subset of Ustari services; it must not bypass the same
verification and authorization rules.

## Design principles

- [ ] Keep the embedded codec `no_std`, allocation-free, and independent of
      HAL, PAC, radio, USB, filesystem, and application types.
- [ ] Use caller-owned fixed-capacity buffers and reject insufficient output
      capacity with typed errors.
- [ ] Validate magic, version, flags, message, channel, sequence, and length
      before reading or dispatching a payload.
- [ ] Bound parser work, reassembly, queue depth, retries, subscriptions,
      response size, and command execution time.
- [ ] Separate read-only diagnostics, privileged state changes, and
      safety-critical actuation into explicit authorization classes.
- [ ] Prevent telemetry, logs, or package transfer from starving control,
      watchdog, recovery, or local safety paths.
- [ ] Keep package authenticity in AMRN and Trust Store policy; Ustari session
      authorization must not become an alternative package-signing mechanism.

## Protocol layering

```text
Physical transport
  -> transport adapter and bounded reassembly
  -> Ustari frame decoder
  -> authenticated session and AEAD validation
  -> sequence and replay protection
  -> capability and authorization policy
  -> diagnostics, lifecycle, telemetry, or control service
```

The reverse path applies the same bounds and policy to responses, telemetry,
and error records. A successfully received frame is not by itself evidence of
authorization or successful command execution; the controller must validate a
typed response or acknowledgement.

## Profiles

### USTARI_FULL

`USTARI_FULL` is the profile for USB CDC, UART, and Wi-Fi/BLE bridges. It uses
the canonical Ustari v1 frame contract from [`ustari-protocol/README.md`](../ustari-protocol/README.md):

- 12-byte fixed header;
- little-endian multi-byte fields;
- bounded variable payload;
- CRC16-CCITT trailer for framing and accidental corruption;
- authenticated session protection for protected commands;
- bounded fragmentation where a transport MTU is smaller than a complete
  frame.

The profile may carry full diagnostics, application lifecycle operations,
versioned telemetry, and authenticated package-transfer messages. Package
transfer remains subject to AMRN validation, signature verification, Trust
Store policy, anti-rollback, atomic activation, and recovery rules.

The profile must support arbitrary byte chunking. One USB or UART read is not
assumed to contain one frame, and a frame must never be dispatched before its
complete length, CRC, and security state are validated.

### USTARI_COMPACT

`USTARI_COMPACT` is the profile for LoRa and ExpressLRS. It preserves Ustari
message meaning, authorization, sequence semantics, and typed errors while
constraining the encoded representation for small and loss-prone packets.

- [ ] Define the compact profile's maximum payload and complete packet size.
- [ ] Define a compact message subset for critical health, status, bounded
      telemetry, acknowledgements, and explicitly approved control requests.
- [ ] Define profile-specific bit packing and field presence rules.
- [ ] Define packet loss, duplicate, ordering, expiry, and bounded retry
      behavior.
- [ ] Define bounded fragmentation only where it is demonstrably safe; do not
      assume a radio can carry a full USTARI_FULL frame.
- [ ] Define duty-cycle, airtime, downlink, latency, and update-rate budgets
      for each radio deployment.
- [ ] Reject package transfer, unrestricted logs, arbitrary shell text, and
      unbounded telemetry from the compact profile.
- [ ] Prove that link loss or reconnect cannot leave an actuator enabled.

Compact encoding reduces Ustari application overhead; it does not remove
radio PHY/MAC headers, FEC, airtime limits, or regulatory constraints. “Zero
overhead” therefore means no unnecessary application-level text or metadata,
not zero bytes on the physical link.

## Frame and message contract

The canonical v1 frame remains defined by `docs/ustari-protocol/README.md` and is
the source of truth for the full profile. Its fixed header carries magic,
version, flags, message ID, channel ID, sequence number, and payload length.
The outer CRC detects accidental corruption only.

- [ ] Freeze the compact profile as a versioned extension without silently
      changing the canonical full-profile header.
- [ ] Define message schemas with maximum size, channel, authentication state,
      authorization class, idempotency, duplicate handling, response, errors,
      and timeout/retry rules.
- [ ] Reserve separate logical channels for session/control,
      diagnostics/telemetry, and package transfer.
- [ ] Define explicit priority and fairness rules; an `URGENT` flag must not
      override authorization or local safety policy.
- [ ] Define compatibility negotiation and rejection of unknown versions,
      flags, channels, and message IDs.

## Security architecture

The security boundary is:

```text
Ustari Decoder
  -> Authenticated Session (AEAD)
  -> Sequence / Replay Protection
  -> Authorization Policy
  -> Service Execution
```

### Decoder

The decoder is a streaming, fixed-capacity state machine. It validates frame
structure and length before payload access, resynchronizes after malformed
input, reports consumed bytes, and never waits indefinitely for more input.
Malformed bytes produce typed errors and cannot cause a panic, heap growth, or
an unbounded loop.

### Authenticated session

Protected control requires a fresh authenticated session. A static token
embedded in an application or shared unchanged across all devices is not an
adequate session design.

- [ ] Specify peer identity, handshake, capability negotiation, and trust
      anchor policy.
- [ ] Derive direction-specific session keys and bind direction and channel
      to the session context.
- [ ] Use an audited `no_std` AEAD implementation; Dali must not implement
      cryptography itself.
- [ ] Authenticate the complete frame header as associated data.
- [ ] Use a fresh per-session nonce prefix and forbid nonce reuse under one
      directional key.
- [ ] Reject encrypted commands before session establishment and invalid
      authentication tags without exposing sensitive details.
- [ ] Keep AMRN package keys separate from Ustari session keys.

### Sequence and replay protection

Every direction has independent sequence state. Sequence wrap requires a new
session or rekey; it must never silently wrap under the same key.

- [ ] Define the replay-window width and bounded state storage.
- [ ] Reject stale, duplicated, skipped-beyond-window, and wrong-direction
      sequence values according to the negotiated policy.
- [ ] Define reboot, reconnect, session expiry, and key-epoch transitions.
- [ ] Define idempotency and duplicate responses for every state-changing
      message.
- [ ] Add host tests for replay, reordering, duplication, wrap, and reconnect.

### Authorization policy

Authorization is evaluated after structural, cryptographic, and replay checks.
The policy is capability- and service-specific, not transport-specific.

- [ ] Define read-only, privileged, package-management, and safety-critical
      capability classes.
- [ ] Require explicit capability negotiation and deny by default.
- [ ] Require confirmation, deadline, and cancellation for dangerous actions.
- [ ] Rate-limit diagnostics, failed authentication, and state-changing
      commands.
- [ ] Record bounded audit events without logging keys, tokens, or secrets.
- [ ] Define behavior when a session expires during command execution.

CRC16 or CRC32 can detect accidental corruption, but neither authenticates a
peer. A CRC32 may be used by a specific transport or package-transfer layer
when its error-detection requirements justify the extra bytes; it must not be
presented as authorization or a replacement for AEAD authentication.

## Local hardware fail-safe

Radio communication is never the sole safety mechanism for motors, ESCs, or
other hazardous actuators. A remote emergency-stop request is useful as a
command, but it is not physically immediate and may be delayed, duplicated,
corrupted, or lost.

- [ ] Keep emergency stop and output disable in a local kernel-owned or
      hardware-owned path.
- [ ] Define local watchdog, command deadline, link-loss timeout, disarm, and
      safe-output behavior.
- [ ] Require an independent arming state, physical interlock, output limit,
      and maximum motor-test duration.
- [ ] Ensure motor/ESC output is disabled on boot, reset, session expiry,
      malformed command, authorization failure, and transport loss.
- [ ] Make `motor test` a separately reviewed service, not a generic shell
      escape hatch.
- [ ] Add hardware evidence for timeout, emergency stop, power-cycle, and
      radio-loss behavior before any production control claim.

The local fail-safe must remain effective when Ustari, the host, the radio,
the scheduler, or the application is unavailable.

## Host-side presentation

The host owns human interaction and visualization:

```text
User / GUI
  -> dali shell or dashboard command model
  -> Ustari encoder
  -> selected transport
  -> Dali OS Ustari service

Dali OS typed response / telemetry
  -> Ustari decoder
  -> host validation and model
  -> text, table, alert, or GUI visualization
```

- [ ] Define `dali shell` local and remote session behavior.
- [ ] Render diagnostics and errors from typed fields, not parsed log text.
- [ ] Provide plain-text output and optional ANSI colors entirely on the host.
- [ ] Implement request IDs, response status, cancellation, and bounded
      retransmission.
- [ ] Define dashboard subscriptions, stale-data indicators, reconnect, and
      resumption behavior.
- [ ] Validate protocol version and capability compatibility before issuing
      commands.

The kernel remains pure binary and zero-heap. Human-readable logs may still
exist as a separate development or recovery facility, but they are not the
native application protocol and cannot be used to infer command success.

## Service mapping

| Service | Ustari role | Required boundary |
| --- | --- | --- |
| Diagnostics | `sysinfo`, `ps`, `mem`, `dmesg`, reset status | Read-only, bounded snapshots and streams |
| Application lifecycle | list, inspect, start, stop | Existing AMRN, Trust Store, MPU, ABI, and lifecycle policy |
| Telemetry | sensor health, orientation, battery, load | Freshness, validity, rate, and subscriber bounds |
| Package transfer | start, chunk, finish, abort | AMRN signatures, anti-rollback, atomic activation, recovery |
| Motor/ESC test | Explicit safety-critical command | Local failsafe, interlock, deadline, output cap, hardware evidence |

## Implementation and acceptance gates

- [ ] Approve the Ustari threat model, message schemas, and profile budgets.
- [ ] Implement and fuzz-test the hardware-neutral full-profile codec.
- [ ] Implement session authentication, AEAD, replay protection, and typed
      authorization failures.
- [ ] Implement USB CDC and UART wired adapters.
- [ ] Deliver wired read-only diagnostics before lifecycle or actuator control.
- [ ] Add host `dali shell` support and typed response validation.
- [ ] Add F405 target evidence for framing errors, authentication failures,
      replay rejection, and bounded command execution.
- [ ] Integrate AMRN lifecycle and package transfer only after wired security
      acceptance.
- [ ] Define and independently validate the compact LoRa/ELRS profile.
- [ ] Validate telemetry loss, stale data, reconnect, and bandwidth budgets.
- [ ] Review and validate local motor/ESC failsafe behavior independently.
- [ ] Never treat USB, UART, Wi-Fi, LoRa, or ExpressLRS connectivity alone as
      authorization or hardware acceptance.

## Explicit non-claims

This document does not claim that Ustari is implemented, that a Dali shell or
dashboard exists, or that LoRa/ExpressLRS control is safe or available. It
does not replace AMRN signing, Trust Store policy, Secure Boot, MPU isolation,
DMA policy, watchdog recovery, or local hardware emergency-stop behavior.
