# Dali OS Interactive Shell, Remote Telemetry, and Control Ecosystem

Status: **Future architecture backlog**.

This document describes a future control and observability ecosystem. It does
not modify the current thirteen-step roadmap, the MVP boot path, the ABI, the
storage layout, the package layout, or the existing security boundaries.
Implementation begins only after the relevant driver, runtime, application
lifecycle, and first-stage bootloader contracts have been explicitly approved.

## Vision and scope

Dali OS should expose a bounded, transport-agnostic interactive shell and a
typed telemetry channel for diagnosis, application lifecycle operations, and
robotics peripheral control. The same kernel-facing protocol should work over
wired USB CDC and UART and over future adapters such as an ESP32 Wi-Fi/BLE
bridge, LoRa, or ExpressLRS radio telemetry.

The system must remain `no_std`, zero-heap in kernel and realtime paths,
bounded in memory and execution time, and explicit about the difference
between observation, privileged control, and safety-critical actuation.
Wireless transports are adapters, not privileged bypasses: every transport
must terminate at the same authenticated command and telemetry boundaries.

## Architectural layers

### 1. Transport-agnostic byte-stream engine

The engine accepts caller-owned byte buffers from physical transports and
exposes a common bounded stream interface. USB CDC, UART, Wi-Fi/BLE bridge,
LoRa, and ExpressLRS adapters must not leak their framing or hardware types
into command parsing or kernel policy.

- [ ] Define a `no_std` byte-stream contract for bounded receive and transmit.
- [ ] Define fixed-capacity RX/TX buffers and explicit overflow behavior.
- [ ] Define partial-read, partial-write, disconnect, backpressure, and
      timeout semantics.
- [ ] Define transport capability metadata without embedding a concrete
      device, port, baud rate, radio, or network address.
- [ ] Add USB CDC and UART adapters behind the existing driver boundaries.
- [ ] Specify an ESP32 Wi-Fi/BLE bridge protocol with explicit framing and
      reconnect behavior.
- [ ] Specify LoRa and ExpressLRS framing, packet limits, loss handling, and
      command acknowledgement requirements.
- [ ] Prove that transport teardown cannot leave a privileged command half
      executed.
- [ ] Add host contract tests for fragmentation, overflow, disconnect, and
      bounded retry behavior.
- [ ] Add target evidence for wired transport delivery before wireless work.

### 2. Zero-heap command parser and dispatcher

The parser consumes a fixed-capacity ring buffer and produces typed command
records backed by caller-owned storage. It must never allocate, block
indefinitely, or interpret an incomplete frame as a command. ANSI color output
is a presentation feature and must not alter command semantics.

- [ ] Define command-line framing, line limits, escaping, whitespace, and
      end-of-input rules.
- [ ] Implement a fixed-capacity ring buffer with overflow recovery and no
      heap allocation.
- [ ] Define a bounded token view representation over input bytes.
- [ ] Define command registration and dispatch tables with compile-time or
      fixed-capacity limits.
- [ ] Define typed parse, authorization, unsupported-command, timeout, and
      execution errors.
- [ ] Define cancellation and deadline propagation for every command.
- [ ] Add optional ANSI color rendering with a plain-text fallback.
- [ ] Add bounded help and completion metadata without dynamic strings.
- [ ] Add host tests for quoting, malformed input, truncation, overflow,
      cancellation, and repeated commands.
- [ ] Add target tests proving parser execution does not allocate or spin
      indefinitely.
- [ ] Define command authorization classes for read-only, privileged, and
      safety-critical operations.

### 3. System diagnostics module

Diagnostics are read-only by default and must report measured values with
explicit validity and support states. Sensitive data, private keys, and raw
unbounded memory dumps must never be exposed through the shell.

- [ ] Define a bounded diagnostics snapshot contract.
- [ ] Implement `sysinfo` for CPU clock frequency, uptime, reset cause, and
      chip temperature where the board provides a supported sensor.
- [ ] Implement `ps` and `tasks` for AMRN task identity, PSP stack usage,
      priority, and `Running`, `Ready`, or `Blocked` state.
- [ ] Implement `free` and `mem` for the zero-heap SRAM region map,
      allocated blocks, free blocks, and measurement validity.
- [ ] Implement `dmesg` for bounded circular-log history retrieval.
- [ ] Add a bounded `dmesg` live-stream mode with subscriber limits and
      backpressure behavior.
- [ ] Implement `reboot` with explicit confirmation and reset-cause policy.
- [ ] Implement `safe-mode` with authorization, confirmation, and recovery
      semantics.
- [ ] Define redaction and rate limits for diagnostic output.
- [ ] Add host tests for formatting, truncation, unavailable metrics, and
      region accounting.
- [ ] Add F405 evidence for each supported metric and reset transition.

### 4. Application lifecycle management

Application commands must use the existing signed AMRN and trust-store
boundaries. A shell command must not create an alternative loader, bypass
anti-rollback, or grant an application kernel-owned resources.

- [ ] Define a read-only application metadata and lifecycle contract.
- [ ] Implement `app list` for valid, signed AMRN packages discovered from
      the approved repository and trust-store path.
- [ ] Implement `app start <name>` only through an authorized lifecycle
      service and the existing loader policy.
- [ ] Implement `app stop <id>` with safe termination, resource revocation,
      and bounded memory cleanup.
- [ ] Implement `app inspect <name>` for verified metadata, memory
      boundaries, entry point, target, ABI, and signature status.
- [ ] Require explicit authorization for start, stop, and other state changes.
- [ ] Define behavior for duplicate names, stale generations, revoked keys,
      expired packages, missing storage, and running dependencies.
- [ ] Define recovery when an application cannot be safely terminated.
- [ ] Add host tests for lifecycle state transitions and authorization errors.
- [ ] Add target evidence for list, inspect, start, stop, and failed-operation
      recovery before exposing remote control.

### 5. Robotics and telemetry engine

Telemetry is observational unless a command is explicitly classified as
control. Motor and ESC operations require an independent safety contract,
bounded duration, arming state, emergency-stop behavior, and a hardware
acceptance gate. No shell command may turn an unverified peripheral into a
production flight-control path.

- [ ] Define typed sensor health, freshness, validity, and availability
      records for IMU, barometer, magnetometer, and GPS.
- [ ] Implement `sensor status` using hardware-neutral sensor contracts.
- [ ] Define a bounded telemetry sample schema for orientation, battery
      voltage, system load, uptime, and sensor health.
- [ ] Define `telemetry stream --freq <hz>` with bounded frequency limits,
      subscriber limits, loss handling, and cancellation.
- [ ] Define binary telemetry framing, schema versioning, endianness, and
      integrity checks before selecting a radio transport.
- [ ] Define motor/ESC safety states, arming prerequisites, output limits,
      maximum test duration, and emergency stop behavior.
- [ ] Implement `motor test --id <n> --speed <pct>` only behind an explicit
      authorization and a board-specific safety adapter.
- [ ] Require physical interlock or equivalent deployment policy before any
      motor output is enabled.
- [ ] Add host tests for sample encoding, stale data, frequency bounds,
      command cancellation, and safety-state transitions.
- [ ] Add target sensor-health evidence before live telemetry acceptance.
- [ ] Add hardware evidence for motor/ESC test isolation, timeout, and stop.

### 6. Host ecosystem integration

The host tooling must consume the same versioned contracts as the target. A
remote CLI or GUI dashboard is not allowed to infer success from transport
delivery alone; it must receive and validate a typed response or telemetry
acknowledgement.

- [ ] Define the `dali shell` local and remote CLI command model.
- [ ] Add transport selection and discovery without hardcoding a physical
      port, radio, device ID, or network endpoint.
- [ ] Define authenticated session establishment, capability negotiation, and
      command authorization for remote sessions.
- [ ] Define request IDs, response status, cancellation, replay protection,
      and bounded retransmission.
- [ ] Implement a versioned binary telemetry protocol for host dashboards.
- [ ] Provide a text rendering of telemetry for terminal use without making
      text the kernel protocol.
- [ ] Define dashboard subscription, disconnect, resume, and stale-data
      behavior.
- [ ] Add host integration tests for CLI parsing, protocol compatibility,
      authentication failures, loss, and reconnect.
- [ ] Add an interoperability fixture that uses only documented contracts and
      caller-owned buffers.
- [ ] Add end-to-end wired target evidence before approving a wireless bridge.
- [ ] Add wireless bridge evidence separately for Wi-Fi/BLE, LoRa, and
      ExpressLRS; passing one transport must not imply the others.

## Command specification backlog

The following commands are planned interfaces, not current capabilities.
Their implementation is gated by the contracts above and must preserve the
existing security and memory boundaries.

### System diagnostics

| Command | Intended behavior | Safety boundary |
| --- | --- | --- |
| `sysinfo` | CPU clock, uptime, reset cause, and supported chip temperature | Read-only, bounded snapshot |
| `ps` / `tasks` | AMRN tasks, PSP usage, priorities, and lifecycle state | No raw stack disclosure; bounded output |
| `free` / `mem` | Zero-heap SRAM regions and allocation accounting | Measured regions only; no unbounded dump |
| `dmesg` | Circular log history or bounded live stream | Rate-limited, redacted, backpressure-aware |
| `reboot` | Authorized software reset | Confirmation and reset-cause recording |
| `safe-mode` | Authorized transition to Safe Recovery Mode | Must preserve watchdog and recovery servicing |

### Application management

| Command | Intended behavior | Safety boundary |
| --- | --- | --- |
| `app list` | List valid signed AMRN packages | Existing trust-store and anti-rollback policy |
| `app start <name>` | Load and start an authorized application | Existing loader, MPU, ABI, and lifecycle policy |
| `app stop <id>` | Stop and revoke an application context | Bounded retirement and resource cleanup |
| `app inspect <name>` | Show verified metadata and memory boundaries | No bypass of signature or package validation |

### Robotics and telemetry

| Command | Intended behavior | Safety boundary |
| --- | --- | --- |
| `sensor status` | Report supported sensor health and readiness | Freshness and validity must be explicit |
| `motor test --id <n> --speed <pct>` | Run a bounded individual motor/ESC test | Authorization, interlock, output cap, timeout, stop |
| `telemetry stream --freq <hz>` | Stream typed live telemetry to a channel | Bounded frequency, subscribers, and loss handling |

## Staged delivery plan

### Phase A — Contracts and threat model

- [ ] Record the shell, telemetry, and control threat model.
- [ ] Define command authorization classes and trust boundaries.
- [ ] Define byte-stream, parser, response, and telemetry schemas.
- [ ] Define memory, queue, CPU, latency, and timeout budgets.
- [ ] Define versioning and compatibility rules for target and host.
- [ ] Obtain architecture approval before modifying kernel or boot paths.

### Phase 5 / Services — Ustari integration entry point

This is the entry point for the future Services phase. The accepted protocol
architecture is recorded in
[`ustari_application_protocol.md`](../architecture/ustari_application_protocol.md).
The first implementation slice is wired, read-only diagnostics; lifecycle
control, telemetry actuation, and wireless transports remain gated by their
own contracts and acceptance evidence.

- [ ] Review and approve the Ustari application protocol contract.
- [ ] Implement the hardware-neutral Ustari full-profile codec and bounded
      session boundary.
- [ ] Integrate Ustari with USB CDC and UART without exposing HAL or PAC types.
- [ ] Implement wired read-only `sysinfo`, `free`, `mem`, `ps`, and `dmesg`.
- [ ] Validate AEAD, replay protection, authorization, and typed error paths.
- [ ] Capture F405 wired diagnostics evidence before enabling remote control.

### Phase B — Wired diagnostics MVP

- [ ] Implement the hardware-neutral stream and parser contracts.
- [ ] Implement the local USB CDC/UART shell adapter.
- [ ] Implement read-only `sysinfo`, `free`, `mem`, `ps`, and `dmesg`.
- [ ] Add bounded responses, cancellation, authorization, and error output.
- [ ] Pass host tests, strict Clippy, formatting, and allocation checks.
- [ ] Capture F405 wired-console evidence for normal boot and Safe Mode.

### Phase C — Authorized application control

- [ ] Integrate lifecycle commands with the existing signed AMRN path.
- [ ] Validate list and inspect before start and stop operations.
- [ ] Add safe termination and recovery evidence on F405.
- [ ] Validate anti-rollback, revocation, expiry, and authorization failures.
- [ ] Do not expose lifecycle control remotely until wired acceptance passes.

### Phase D — Binary telemetry and dashboard

- [ ] Freeze the versioned telemetry schema and integrity rules.
- [ ] Implement bounded target streaming and host decoding.
- [ ] Implement `dali shell` remote session negotiation and acknowledgements.
- [ ] Build a dashboard integration against the documented binary protocol.
- [ ] Capture wired loss, reconnect, stale-data, and backpressure evidence.

### Phase E — Robotics control and wireless adapters

- [ ] Integrate sensor contracts and health reporting.
- [ ] Validate telemetry freshness and system-load behavior under bounded load.
- [ ] Implement motor/ESC test controls only after safety approval.
- [ ] Validate physical interlocks, timeout, emergency stop, and failure modes.
- [ ] Add ESP32 Wi-Fi/BLE bridge integration and independent acceptance.
- [ ] Add LoRa integration and independent acceptance.
- [ ] Add ExpressLRS integration and independent acceptance.
- [ ] Prove that wireless loss or reconnect cannot leave actuation enabled.

## Acceptance gates

- [ ] Every kernel path remains `no_std`, zero-heap, fixed-capacity, and
      bounded.
- [ ] Every command has typed authorization, timeout, cancellation, and error
      behavior.
- [ ] Every transport has explicit framing, loss, reconnect, and backpressure
      semantics.
- [ ] Read-only diagnostics are validated separately from state-changing
      commands.
- [ ] Application control reuses the existing AMRN, trust-store,
      anti-rollback, MPU, and lifecycle boundaries.
- [ ] Motor/ESC control has a separate safety review and physical acceptance.
- [ ] Host tests, strict target Clippy, release builds, and `git diff --check`
      pass for each slice.
- [ ] F405 evidence records the firmware revision, transport, wiring,
      expected trace, observed trace, and result.
- [ ] Wireless transports are accepted independently and are never inferred
      from wired transport results.

## Explicit non-claims

This roadmap does not currently provide a shell, remote CLI, telemetry
protocol, dashboard, sensor integration, motor control, wireless bridge, or
flight-control capability. It does not authorize bypassing the existing
loader, trust-store, watchdog, MPU, DMA, storage, or bootloader contracts.
