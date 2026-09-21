# 15. Planned crate boundaries

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
cartridge, lifecycle, logging, and authorization services without moving board
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

### U4 — Cartridge delivery

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
