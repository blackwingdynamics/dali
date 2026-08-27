# 1. Status and authority

Ustari is the planned binary control, telemetry, and package-delivery protocol
for Dali OS. This document is its design contract and source of truth.

The current status is **design-only**. Ustari is not required for Dali OS
0.1.0, and the existing USB CDC byte stream is not an Ustari implementation.
The protocol must not be advertised as available until its implementation and
the acceptance gates in this document exist.

The initial protocol version is `USTARI_VERSION = 1`. A change to the frame
layout, byte order, authentication model, replay rules, or command semantics
requires a versioning decision and an update to this document first.

Ustari is separate from AMRN and package signing:

- AMRN signatures establish whether a package is trusted and installable.
- An Ustari session establishes whether a peer may issue a live command.
- A package signing key is not an Ustari session key.
- A transport connection is not an authenticated session.

## 2. Goals and non-goals

Ustari provides a bounded command, telemetry, and package-delivery protocol
between Dali OS and host tools such as the Dali CLI or Dali Studio.

Its goals are:

1. A `#![no_std]` embedded core with no heap allocation.
2. A bounded streaming decoder that can recover from malformed input.
3. A transport-independent frame contract.
4. Authenticated control commands with explicit authorization policy.
5. Replay protection with a sequence number large enough for a live session.
6. Bounded fragmentation for transports with a smaller MTU.
7. Safe package transfer with atomic activation and rollback.
8. A host implementation using the same codec and validation rules.

Ustari does not guarantee any of the following by itself:

- A remote emergency stop is not physically immediate. Safety-critical
  actuators require a local hardware or kernel-owned stop path.
- CRC is not authentication and does not replace a signature or AEAD tag.
- USB enumeration, a serial connection, or a valid frame is not authorization.
- Ustari does not replace AMRN validation, package signatures, Trust Store
  policy, Secure Boot, or the kernel memory-isolation boundary.
- Reserved commands do not mean that compression, radio transport, concurrent
  application control, or OTA activation is implemented.

## 3. Resource and realtime contract

The embedded core uses caller-owned storage. It must not allocate from the
heap, grow an unbounded buffer, or wait indefinitely for a peer.

The implementation must define named configuration values for:

- `MAX_PAYLOAD_LEN = 1024` bytes;
- maximum complete frame length;
- maximum reassembly count and reassembly timeout;
- maximum outstanding file transfers;
- replay-window width;
- per-command execution and response limits.

These values are protocol or target configuration, not unexplained literals in
implementation code. A profile may lower a limit but may not silently raise it
beyond the negotiated and compiled contract.

Malformed external bytes must return typed errors and must not cause a panic,
unbounded loop, or access outside caller-provided buffers.

## 4. Participants and sessions

- **Device** — the Dali OS endpoint. It owns authorization, package policy,
  safety policy, and session state.
- **Controller** — a host, service, or remote peer. A controller is not
  trusted merely because it uses USB.
- **Session** — an authenticated, bounded lifetime between one device and one
  controller. It owns keys, sequence state, capabilities, and authorization.
- **Channel** — a logical traffic class inside a session.

The device must establish a fresh authenticated session before accepting
protected control commands. The handshake and identity-authorization record
must be specified as a separate security sub-contract before crypto code is
implemented. A static shared secret embedded in every application, or a
private key transmitted in a frame, is forbidden.
