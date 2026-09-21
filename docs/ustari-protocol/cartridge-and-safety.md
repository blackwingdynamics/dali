# 13. Cartridge transfer and activation

The transfer lifecycle is:

1. `FILE_START` declares transfer ID, cartridge ID, format/version, exact length,
   digest, and signature metadata.
2. The device validates limits and authorization before reserving a transfer.
3. `FILE_CHUNK` carries a bounded offset or in-order chunk number.
4. Bytes go only to a temporary object or inactive slot.
5. `FILE_FINISH` requires exact length, digest, AMRN validation, signature and
   Trust Store validation, target/ABI compatibility, and policy checks.
6. Candidate/commit-marker activation makes power loss recoverable.
7. Activation is reported only after read-back and commit validation.
8. Failure leaves the active cartridge unchanged.

The initial profile requires in-order chunks and one transfer per peer.
Resumable or out-of-order transfers need an explicit storage and replay design.
No filename or UI label is trusted as cartridge identity.

## 14. Safety and emergency commands

An `EMERGENCY_STOP` message may coordinate device behavior, but USB, UART, CAN,
LoRa, Bluetooth, and Wi-Fi cannot provide a universal real-time guarantee.
Link latency, buffering, disconnects, and power loss are outside the protocol.

Safety-relevant hardware must provide a local kernel-owned stop path with a
defined electrical and timing contract. Ustari is an additional request path,
never the sole certified safety function.
