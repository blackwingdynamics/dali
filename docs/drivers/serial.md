# UART and SPI

UART and SPI contracts are synchronous interfaces with explicit bounded
timeouts. `SerialRead` and `SerialWrite` accept caller-owned byte slices;
`SpiTransfer` uses one caller-owned mutable slice for a full-duplex exchange.
No contract allocates, retains a caller buffer, or waits indefinitely.

## Buffer rules

- Read operations write at most `buffer.len()` bytes and return the accepted
  count.
- Write operations return the accepted count; partial progress is normal and
  can be resumed by the caller.
- SPI transfers reject buffers that cannot represent the configured response
  with `InvalidBuffer`.
- Zero-length and otherwise invalid operations are rejected by the adapter's
  typed contract rules rather than being silently expanded.

## Progress and errors

Adapters return `WouldBlock` when the peripheral cannot make progress without
waiting, and `Timeout` when the supplied `Duration` is outside the adapter's
bound. UART framing, parity, and overrun conditions map to typed errors. SPI
NACK and arbitration loss remain distinguishable from transport absence or a
generic hardware fault.

PAC, HAL, baud, bus, chip-select, and register details belong to a platform
adapter. The generic crate exposes none of those target-specific types. Fixed-
size host mocks validate partial transfers, buffer limits, timeout handling,
disconnects, and typed protocol failures; they do not provide hardware
acceptance evidence.
