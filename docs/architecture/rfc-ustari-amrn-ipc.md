# RFC: Ustari-Based AMRN IPC Messaging

Status: **Proposed future architecture; implementation not started**.

## Summary

This RFC proposes Ustari as the typed application-layer IPC language between
an AMRN application and the Dali kernel SVC gateway. The proposal does not
replace the existing SVC ABI, AMRN package format, Trust Store, MPU policy, or
platform driver contracts. It defines a future message boundary that can be
implemented above those mechanisms.

```text
AMRN application
  -> fixed-capacity Ustari message
  -> kernel-owned shared-memory ring
  -> SVC gateway validation and authorization
  -> kernel service
```

Responses use the same path in reverse. A Ustari message is an IPC envelope;
it is not automatically trusted merely because the sender is an installed
application.

## Motivation

The current application contract exposes typed SVC entry points. As Dali grows
to multiple isolated applications and service domains, each service needs a
uniform representation for requests, responses, errors, capabilities, and
bounded asynchronous work. Reusing Ustari gives local IPC and remote control a
common semantic model without making the kernel parse human-oriented CLI text.

The design is intended to preserve the embedded constraints already required
by Dali:

- `no_std` and zero heap allocation in the kernel and application IPC paths;
- fixed headers, caller-owned buffers, and bounded message sizes;
- explicit authorization and typed failure responses;
- compatibility with MPU-protected shared memory and kernel-owned SVC entry;
- transport independence for a future local, wired, or multi-node deployment.

## Scope and non-goals

This RFC covers the future IPC message model, its memory boundary, and its
relationship to SVC. It does not authorize an ABI change or claim that any of
the following is implemented:

- a multi-application production scheduler or IPC runtime;
- a Ustari codec in the kernel or an AMRN SDK;
- shared-memory MPU mappings or zero-copy rings;
- authenticated sessions for local application IPC;
- remote networking, telemetry, or actuator control;
- replacement of the current ABI v2/v3 contracts.

The implementation must be added as a separately reviewed roadmap slice with
updated ABI/versioning and hardware-neutral host tests.

## Proposed boundary

### Application side

An AMRN application submits a bounded request through an SDK wrapper. The
wrapper serializes a request into a caller-owned buffer or reserves one slot
in a kernel-created ring. The application cannot choose kernel addresses,
modify ring metadata, or invoke arbitrary service identifiers.

The application sees typed status values and bounded response data. It does
not receive a raw MMIO handle, a kernel pointer, or a direct interrupt path.

### SVC gateway side

The SVC gateway remains the privileged entry point. It validates the request
before service dispatch:

1. validate the SVC number and application context;
2. validate the ring slot, message length, alignment, and ownership;
3. decode the Ustari header and reject unsupported versions or flags;
4. validate message-specific fields and bounded payload ranges;
5. apply capability and service authorization;
6. dispatch only to a kernel-owned service adapter;
7. write a typed response and publish it with an ordered commit operation.

The gateway must fail closed. Malformed, stale, unauthorized, or out-of-range
messages must not reach service code and must not cause an unbounded retry.

## Ustari message profile for local IPC

The local IPC profile should reuse the semantic message identifiers and typed
error model from [`ustari-protocol/README.md`](../ustari-protocol/README.md), while allowing
a local framing profile instead of a physical transport frame.

The eventual profile must define:

- a fixed header version and maximum payload size;
- request, response, error, cancellation, and notification message classes;
- service identifier, operation identifier, request ID, and sequence fields;
- capability class and required authorization state;
- alignment, slot ownership, and completion-state rules;
- bounded queue depth, response size, and service execution time;
- duplicate, cancellation, timeout, and application-termination behavior.

The profile must not use raw text, dynamic strings, or unbounded serialization.
Human-readable diagnostics remain a host presentation concern.

## MPU-protected zero-copy ring buffers

The preferred future transport is a pair of bounded single-producer/single-
consumer rings in a shared MPU region: one request ring and one response ring.
The exact topology is intentionally left for the implementation slice.

```text
Application-owned request bytes
  -> application writable / kernel readable ring region

Kernel-owned response bytes
  -> kernel writable / application readable ring region
```

The MPU contract must enforce direction and ownership. The application must
not be able to write response metadata, kernel ring indices, or another
application's slots. The kernel must validate every index and length against
the declared ring capacity even when the MPU configuration is active.

Publication requires explicit ordering semantics: write payload, write the
message header, then publish the producer index. Consumption must validate the
published slot before advancing the consumer index. Reset and application
retirement must invalidate outstanding slots without allowing stale messages
to be consumed by a replacement context.

Zero-copy is an optimization, not a validation exemption. If an operation
requires transformation, authentication, or a lifetime that exceeds the ring
slot, the kernel may use a bounded copy into kernel-owned storage.

## Benefits and trade-offs

### Benefits

- **Unified messaging:** local SVC IPC and future Ustari transports can share
  schemas, status values, request IDs, and authorization classes.
- **Network and multi-node readiness:** service messages can later cross a
  transport adapter without making the kernel depend on USB, UART, radio, or
  a network stack.
- **Zero-copy compatibility:** fixed-capacity Ustari payloads map naturally to
  MPU-protected shared-memory slots.
- **Observability:** typed request/response records are easier for host tools
  and dashboards to validate than parsed log text.
- **Policy consistency:** authorization, replay/sequence policy, and safety
  classes can be represented consistently across local and remote boundaries.

### Trade-offs

- **Latency:** decoding, validation, authorization, and ring publication add
  work compared with a direct fixed-argument SVC call.
- **Generality cost:** a universal envelope can be larger and slower than a
  service-specific ABI for the smallest operations.
- **Memory pressure:** ring slots and bounded response storage consume scarce
  SRAM and must be included in the measured memory budget.
- **Complexity:** version negotiation, cancellation, ownership, and fault
  recovery create more states than a synchronous SVC call.
- **Security cost:** zero-copy shared memory expands the attack surface around
  indices, lifetimes, stale data, and MPU permissions.
- **Debugging:** binary messages require reliable host decoding and protocol
  inspection tools; logs alone are insufficient for diagnosis.

The implementation should use direct typed SVC calls for latency-critical
internal primitives where a Ustari envelope provides no policy or evolution
benefit. Ustari should be the service-facing message language, not a mandate
to encode every CPU or context-switch operation.

## Security and failure model

Local IPC does not automatically require remote-style AEAD, but it does
require context identity, ownership validation, capability checks, and replay
or stale-slot protection. If a message crosses an untrusted boundary, the
authenticated-session and sequence protections defined by Ustari become
mandatory.

The gateway must reject:

- messages from retired or non-running application contexts;
- invalid slot ownership, indices, lengths, alignment, and message versions;
- unsupported operations, capabilities, or authorization classes;
- duplicate or stale requests where the operation is not idempotent;
- requests that exceed their bounded execution or response budget.

On application fault, reset, watchdog recovery, or ring corruption, the
kernel must retire or invalidate the affected context and return services to a
safe state. Safety-critical outputs must not depend on an application or IPC
ring continuing to run.

## Staged implementation plan

1. Freeze a hardware-neutral local Ustari message schema and typed error set.
2. Add host codec tests for malformed headers, bounds, sequence, cancellation,
   duplicate requests, and fixed-capacity ring behavior.
3. Define the ABI/versioning amendment for an SVC-to-Ustari gateway.
4. Specify MPU permissions, ring ownership, publication ordering, and reset
   invalidation rules for one isolated application.
5. Implement a read-only diagnostic service before lifecycle or actuator
   operations.
6. Add target evidence for unauthorized access, stale slots, application
   retirement, ring corruption, and bounded service execution.
7. Only then evaluate remote transport reuse and multi-node routing.

## Decision record

This RFC records Ustari as the proposed language between future AMRN
applications and the kernel SVC gateway. It is compatible with the existing
transport-agnostic Ustari vision, but it does not change current code or
promote the experimental ABI v3 path to production multi-application IPC.
