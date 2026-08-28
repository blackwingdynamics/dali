# 17. Acceptance gates

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
