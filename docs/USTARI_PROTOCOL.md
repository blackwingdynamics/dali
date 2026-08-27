# Ustari Protocol

## 1. Status and authority

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

## 5. Canonical frame format

Version 1 uses a **12-byte fixed header**, a variable payload, and a 2-byte
CRC16 trailer. The header is 12 bytes because replay protection uses a `u32`
sequence number; the previously proposed 10-byte header could not contain that
field without an unsafe trade-off.

```text
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  MAGIC[0]     |  MAGIC[1]     | VERSION       | FLAGS         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| MSG_ID        | CHANNEL_ID    |       SEQUENCE_NUM            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|       SEQUENCE_NUM            |       PAYLOAD_LEN             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         PAYLOAD                               |
|                    0 .. MAX_PAYLOAD_LEN                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         CRC16-CCITT                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

All multi-byte integers are little-endian. Magic is a byte sequence, not an
integer whose representation depends on host endianness.

| Offset | Field | Size | Contract |
| --- | --- | ---: | --- |
| `0x00` | `MAGIC[0..2]` | 2 | `0xDA, 0x11` |
| `0x02` | `VERSION` | 1 | `1` |
| `0x03` | `FLAGS` | 1 | Defined bits only |
| `0x04` | `MSG_ID` | 1 | Typed message identifier |
| `0x05` | `CHANNEL_ID` | 1 | Typed logical channel |
| `0x06` | `SEQUENCE_NUM` | 4 | `u32`, per session and direction |
| `0x0A` | `PAYLOAD_LEN` | 2 | `u16`, `0..=1024` |
| `0x0C` | `PAYLOAD` | N | Plaintext or ciphertext plus tag |
| `0x0C + N` | `CRC16` | 2 | CRC16-CCITT, little-endian |

The total length is `HEADER_LEN + PAYLOAD_LEN + CRC_LEN`. An encrypted payload
includes its authentication tag in `PAYLOAD_LEN`, so its maximum plaintext is
lower than `MAX_PAYLOAD_LEN`.

CRC parameters:

- polynomial `0x1021`;
- initial value `0xFFFF`;
- input and output reflection disabled;
- final XOR `0x0000`;
- serialized little-endian.

CRC detects framing and accidental corruption. It proves neither origin nor
authorization and provides no confidentiality.

## 6. Flags and channels

| Bit | Name | Meaning |
| ---: | --- | --- |
| `0x01` | `ENCRYPTED` | Payload is encrypted and authenticated for the session |
| `0x02` | `ACK_REQUIRED` | Receiver returns an authenticated result |
| `0x04` | `URGENT` | Bounded priority handling is requested |
| `0x08` | `COMPRESSED` | A separately negotiated bounded codec is used |

Bits `0x10..=0x80` are reserved and must be rejected. The encrypted flag is
not a security policy by itself: after session establishment, control,
package-management, and authorization messages require encryption. Cleartext
telemetry is allowed only by explicit device policy.

Compression occurs before encryption and requires a bounded, negotiated codec.
The flag alone must never select an unbounded decompressor.

Initial channels are `0` control/session, `1` telemetry/diagnostics, and `2`
package transfer. Unknown channels must be rejected with a typed `NACK`.

## 7. Message identifiers and schemas

| Range | Category |
| --- | --- |
| `0x00..=0x0F` | Session and system control |
| `0x10..=0x1F` | Application lifecycle |
| `0x20..=0x2F` | Telemetry and parameters |
| `0x30..=0x3F` | Package transfer |
| `0xF0..=0xFF` | Safety and emergency handling |

Reserved initial names include `PING`, `PONG`, `ACK`, `NACK`, application list,
start/stop/pause/resume, typed telemetry and parameter access, package
start/chunk/finish/abort, and emergency-stop request/result.

Names are not implementations. Every message requires a versioned, bounded
binary schema defining channel, authentication state, maximum size,
authorization, idempotency, duplicate behavior, success response, typed
errors, and timeout/retry semantics.

Application commands identify packages by signed package identity, version,
slot, or runtime identity. They must never authorize a hardcoded host path such
as `/missions/agro.amrn`, and a remote path is never package identity.

## 8. Streaming codec contract

The planned core is a `#![no_std]` crate independent of hardware and kernel:

```rust
pub struct UstariFrame<'a> {
    pub flags: FrameFlags,
    pub message: MessageId,
    pub channel: ChannelId,
    pub sequence: u32,
    pub payload: &'a [u8],
}

impl<'a> UstariFrame<'a> {
    pub fn parse(buffer: &'a [u8]) -> Result<Self, UstariError>;
    pub fn serialize(&self, output: &mut [u8]) -> Result<usize, UstariError>;
}
```

The final API may use const generics or configuration types, but must preserve:

- zero-copy parsing when the caller owns a complete frame buffer;
- a fixed-capacity incremental state machine;
- resynchronization after bad magic, version, length, flags, or CRC;
- length validation before payload access;
- consumed/discarded byte reporting;
- no infinite wait for input;
- serialization failure when output capacity is insufficient;
- errors that distinguish framing, security, authorization, transport, and
  application-policy failures.

Host tests may use caller-owned slices for this hardware-neutral codec. They
are not USB, radio, or kernel hardware evidence.

## 9. Transport adaptation and fragmentation

The core consumes complete frames. Adapters own byte-stream behavior, MTU,
physical packet boundaries, retries, and fragmentation.

USB CDC, UART, and RS-485 adapters feed arbitrary byte chunks to the parser;
one read is never assumed to equal one frame. CAN, LoRa, and other small-MTU
adapters fragment complete frames using their own versioned metadata, which is
not part of the Ustari header.

Each adapter must bound complete frame size, fragment count/offset, reassembly
timeout, simultaneous reassemblies, duplicate/missing fragments, cancellation,
and resource exhaustion. The initial profile uses one in-flight reassembly per
peer. Out-of-order and concurrent reassembly require a separate resource plan.

## 10. Session authentication and encryption

The planned AEAD is ChaCha20-Poly1305 behind an audited, `no_std`-compatible
facade. The exact dependency must be reviewed for maintenance, licensing,
auditability, transitive dependencies, code size, and target support. Dali must
not implement cryptography itself.

For an established session:

1. The handshake authenticates the peer and negotiates capabilities.
2. Direction-specific session keys are derived.
3. The complete 12-byte header is AEAD associated data.
4. The payload is encrypted and receives a 16-byte Poly1305 tag.
5. Ciphertext plus tag is also covered by the outer CRC.

The reserved 12-byte nonce construction is:

```text
nonce = session_nonce_prefix[8] || sequence_num[4]
```

The prefix must be fresh and unpredictable for every authenticated session and
must never be reused with the same directional key. Direction and channel are
bound into key derivation and the authenticated header. Sequence wrap requires
rekey or a new session; wrapping under the same key is forbidden.

Invalid tags, unknown key epochs, and encrypted frames before session
establishment are rejected without dispatch. Failure handling is bounded and
must not log keys or sensitive cryptographic state.

## 11. Replay protection

Replay state is per authenticated session, direction, and channel. The initial
receiver uses a 64-frame sliding window:

- newer sequence values advance the window;
- an unseen value inside the window is accepted once;
- duplicates are rejected;
- values older than the window are rejected;
- unreasonable forward jumps are rejected by bounded session policy.

New sessions reset replay state because they also change keys and nonce prefix.
Commands that must survive reboot require a future durable anti-replay design;
a volatile window alone is insufficient.

## 12. Authorization model

Authentication answers which peer owns a session. Authorization answers what it
may do. The device enforces both for telemetry, parameters, lifecycle,
package upload/activation, Trust Store administration, and safety commands.

Package identity, developer delegation, Trust Store updates, and package
signatures remain governed by `docs/package-distribution/README.md` and AMRN. Ustari
may transport these operations but may not weaken their validation or make a
transport connection an enrollment authority.

Protected results are authenticated. `ACK` means the request was accepted by
device policy, not that a future physical action has already completed.
Long-running work needs an operation identifier and bounded status reporting.

## 13. Package transfer and activation

The transfer lifecycle is:

1. `FILE_START` declares transfer ID, package ID, format/version, exact length,
   digest, and signature metadata.
2. The device validates limits and authorization before reserving a transfer.
3. `FILE_CHUNK` carries a bounded offset or in-order chunk number.
4. Bytes go only to a temporary object or inactive slot.
5. `FILE_FINISH` requires exact length, digest, AMRN validation, signature and
   Trust Store validation, target/ABI compatibility, and policy checks.
6. Candidate/commit-marker activation makes power loss recoverable.
7. Activation is reported only after read-back and commit validation.
8. Failure leaves the active package unchanged.

The initial profile requires in-order chunks and one transfer per peer.
Resumable or out-of-order transfers need an explicit storage and replay design.
No filename or UI label is trusted as package identity.

## 14. Safety and emergency commands

An `EMERGENCY_STOP` message may coordinate device behavior, but USB, UART, CAN,
LoRa, Bluetooth, and Wi-Fi cannot provide a universal real-time guarantee.
Link latency, buffering, disconnects, and power loss are outside the protocol.

Safety-relevant hardware must provide a local kernel-owned stop path with a
defined electrical and timing contract. Ustari is an additional request path,
never the sole certified safety function.

## 15. Planned crate boundaries

The first implementation starts as one focused hardware-neutral crate:

```text
crates/dali-ustari/
  src/
    lib.rs        # public contract
    frame.rs      # header, flags, IDs, serialization
    parser.rs     # bounded incremental decoder
    schema.rs     # typed message payloads
    error.rs      # typed errors
    replay.rs     # bounded sliding window
    security.rs   # session facade and contracts
    fragment.rs   # bounded reassembly primitives
```

It remains `#![no_std]` and must not depend on a board, filesystem, kernel,
USB, or host UI. Host transport and CLI integration remain outside the core.
Crypto stays behind a narrow facade. Kernel integration connects transport,
package, lifecycle, logging, and authorization services without moving board
MMIO or private-key custody into the protocol crate.

## 16. Implementation phases and evidence

Ustari is scheduled after the Dali 0.1.0 release boundary:

### U0 — Design contract

Keep this document synchronized with wire and security decisions. Resolve
header, nonce, replay, authorization, fragmentation, and activation questions
before implementation.

### U1 — Hardware-neutral codec

Implement bounded serialization, incremental parsing, CRC, and typed schemas.
Add host tests for round trips, truncation, malformed lengths, bad flags,
resynchronization, CRC failure, and bounded arbitrary input.

### U2 — Session security

Select and review the crypto dependency. Implement authenticated handshake,
directional keys, nonce lifecycle, authorization context, and replay tests for
bad tags, wrong sessions, duplicates, old values, advancement, and reset.

### U3 — First transport integration

Integrate USB CDC only after U1/U2. Add authenticated `PING`, `PONG`, `ACK`,
`NACK`, and read-only telemetry. Record real F405 evidence separately from
host codec tests.

### U4 — Package delivery

Connect bounded transfer to AMRN, signatures, Trust Store policy,
candidate/commit-marker activation, read-back, and rollback. Require real
storage and power-loss evidence before claiming OTA or remote installation.

### U5 — Additional transports

Add UART/RS-485, CAN, or radio adapters one at a time, each with separate MTU,
fragmentation, timeout, retry, and resource evidence.

### U6 — Safety integration

Define the board-level local stop contract, then integrate and test Ustari as an
additional request path. Validate loss, disconnect, reset, latency, and local
stop behavior on real hardware.

## 17. Acceptance gates

| Layer | Required evidence |
| --- | --- |
| Codec | Host tests and bounded malformed-input coverage |
| Session security | Authentication, nonce, authorization, and replay tests |
| USB integration | Real F405 USB CDC command/response evidence |
| Package delivery | Real storage read-back, signature, atomic activation, rollback evidence |
| Other transports | Separate transport-specific evidence |
| Safety | Board-level local-stop evidence; remote command alone is insufficient |

Compilation, successful flashing, device enumeration, or valid CRC must never
be reported as Ustari acceptance.

## 18. Compatibility and change policy

Unknown versions, flags, channels, message IDs, codecs, crypto suites, and
schema fields fail closed with bounded typed errors. There is no silent
downgrade from authenticated to unauthenticated control.

Backward-compatible additions require explicit capability and schema rules.
Breaking changes require a new protocol version and migration tests. The
implementation must use named constants from this document rather than
duplicating protocol values as unexplained literals.
