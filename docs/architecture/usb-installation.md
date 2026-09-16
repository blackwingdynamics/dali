# USB cartridge installation boundary

## Decision

Cartridge installation uses a dedicated USB bulk interface. The existing CDC
interface remains a log-only output channel and never interprets host input as
installation data. The bulk interface is used because the F405 USB FS
peripheral cannot expose two full CDC-ACM classes within its endpoint budget.

The installation path has four owners:

1. the board USB adapter owns the USB device, CDC/bulk interfaces, endpoints, and
   bounded byte transfer;
2. the transport queue owns framing assembly and overflow accounting;
3. kernel main context owns protocol dispatch, AMRN validation, and failure
   transitions;
4. the target artifact backend owns erase, writes, publication marker, and
   read-back verification.

The artifact installer is an optional board capability separate from the
ordinary board lifecycle. A selected backend exposes it only when the
storage-write composition is enabled; firmware compositions without that
capability retain the existing boot and SD fallback contracts.

The USB interrupt may poll the two CDC classes and copy bounded installer bytes
into the transport queue. It must not parse AMRN data, erase or write Flash,
run cryptographic validation, or publish an artifact. Logging remains an
independent output drain and must not share installer input state.

## Host protocol boundary

The existing DINS v1 frame format remains the transport framing contract. The
installer endpoint is selected by USB interface identity, not by a magic byte
or a command sent through the logging terminal. The host tool must receive a
bounded acknowledgement for each lifecycle command before continuing.

The host discovery layer identifies the installer by the USB vendor/product
identity declared in the target manifest and then requires the vendor-specific
installer interface. A generic vendor-specific interface on an unrelated USB
device is not treated as a Dali installer. Discovery reports the endpoint as a
separate bulk transport with an install capability; it does not claim the
interface or transfer payloads.

Each acknowledgement reuses the DINS frame envelope and contains exactly
seven payload bytes: status, little-endian detail code, and little-endian next
accepted offset. The response echoes the command. A host must reject a
response with an unexpected command, status, payload length, CRC, or offset;
timeouts and rejected responses abort the current candidate.

The first hardware milestone supports one sequential candidate of at most the
target's configured 64 KiB logical cartridge capacity. The sequence is:

1. `Begin` invalidates the current Flash publication marker;
2. bounded `Data` frames fill the candidate sequentially;
3. `Validate` causes main-context AMRN validation and integrity checks;
4. `Commit` writes and verifies the publication marker only after validation;
5. any transport, validation, timeout, or write failure aborts the candidate.

The F405 single-slot limitation remains explicit: an interrupted replacement
may leave Flash empty and the documented SD fallback remains authoritative.
This interface does not provide A/B atomicity.

## Implementation gates

The current `usb-install` build feature represents only the bounded ingress
slice: a dedicated CDC input is copied into the transport queue. It does not
yet expose a cartridge installation command, dispatch frames to the Flash
writer, or claim target installation support.

- host tests must cover the installer response and timeout state machine;
- the F405 adapter must prove that logging CDC remains functional while the
  installer bulk interface receives bounded data;
- no Flash operation may execute from the USB interrupt;
- a target test must install a real AMRN, reboot without SD, and observe its
  execution;
- repeated installation and interrupted-transfer evidence must be recorded
  before enabling the feature in the default development profile.
