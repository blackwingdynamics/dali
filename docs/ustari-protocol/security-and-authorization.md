# 10. Session authentication and encryption

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
cartridge upload/activation, Trust Store administration, and safety commands.

Cartridge identity, developer delegation, Trust Store updates, and cartridge
signatures remain governed by `docs/cartridge-distribution/README.md` and AMRN. Ustari
may transport these operations but may not weaken their validation or make a
transport connection an enrollment authority.

Protected results are authenticated. `ACK` means the request was accepted by
device policy, not that a future physical action has already completed.
Long-running work needs an operation identifier and bounded status reporting.
