# 8. Streaming codec contract

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
