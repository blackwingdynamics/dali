# 5. Canonical frame format

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
