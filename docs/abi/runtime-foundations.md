# Execution mode

- Kernel bootstrap starts privileged and owns the MSP.
- An application runs in unprivileged Thread mode and owns a PSP-backed stack.
- Exception handlers execute privileged code on the MSP.
- The application never receives a pointer to kernel code or kernel-owned
  writable memory.
- The application entry point and return behavior must be replaced by an
  explicit v3 launch frame; the v2 entry function is not compatible.

## Application lifecycle foundation

The kernel runtime defines a bounded lifecycle record for one discovered
cartridge and its manifest-owned slot:

```text
Discovered -> Loaded -> Ready -> Running -> Faulted -> Recovering -> Terminated
```

Each transition is explicit and invalid transitions are rejected. A declared
slot must match the slot allocation recorded at load time. `Terminated` is a
terminal state: the current runtime does not implicitly restart an application
or expose general lifecycle orchestration. The feature-gated scheduler can
explicitly select another declared ready context after a supported handoff or
fault recovery; production lifecycle and protection policy remain separate
contracts.

### Active context ownership foundation

The lifecycle owner tracks at most one currently active application context. A
ready lifecycle must be explicitly activated before it can enter `Running`,
and a second activation through that owner is rejected. The active context
retains the cartridge identity and manifest-owned slot allocation. Retirement is
accepted only after the lifecycle reaches `Terminated`; there is no implicit
restart. The feature-gated scheduler can explicitly select another declared
ready context after a supported handoff or fault recovery.
The v4 loader creates the lifecycle after validated copy and relocation, then
the launch path performs the `Loaded -> Ready -> Running` transition before
MPU activation. Legacy v2/v3 cartridge paths retain their existing launch
behavior until they receive an identity-aware lifecycle contract.
F405 hardware has confirmed these transitions together with two-cartridge
loading, manifest slot boundaries, slot0 execution, and the complete fault
transition. The current policy requires a manual reset after termination,
provides no rollback on the read-only cartridge boundary, and does not arm a
watchdog without a bounded feed owner.

### Context-switch foundation

The runtime defines a hardware-neutral saved-context record containing the
application PSP, `r4..r11`, `CONTROL`, and `EXC_RETURN`. Each scheduler record
also retains the manifest-owned application slot that accompanies the CPU
state during a protected switch. A fixed-capacity context table enforces one
running owner, bounded insertion, terminal exclusion, and round-robin-style
selection of ready contexts. The feature-gated F405 path wires SysTick to
bounded preemption requests, PendSV to save/restore, and slot selection to
MPU reprogramming. F405 silicon has hardware-verified repeated PSP context
switching and slot-specific MPU region switching.

The scheduling contract also defines a board-independent tick budget. Each
target profile declares the platform timer frequency and the quantum in timer
ticks. The budget emits a one-shot PendSV request when its quantum expires, and
the deferred handler remains responsible for the actual register save/restore.

The optional `abi-context-switch` feature provides target-compiled ARM
save/restore primitives for the kernel-owned context record. The primitives do
not select a context, validate a PSP, program the MPU, or enable an interrupt;
those responsibilities remain in the scheduler and platform layers. The
feature-gated path is separate from the default ABI v2 launch path.

The scheduler facade now owns the bounded sequence around those primitives:
SysTick records a preemption request, PendSV saves the active record, and the
facade selects the next ready context. A feature-gated target SysTick hook now
feeds the scheduler and only pends PendSV when both active and ready contexts
exist. The selected platform enables SysTick only after the first application
context is active. Fault recovery permanently retires the active scheduler
context and can select the next ready record; the ARM recovery transfer and
MPU reprogramming still require target evidence. F405 hardware has verified
repeated CPU context switching with independent slot progress markers, but
has now also verified faulted-context exclusion and continued slot0 execution
after slot1 recovery; F405 GDB evidence also verified the slot-specific MPU
code/data bases at `restore_selected`. Bidirectional CPU-side application
memory rejection is hardware-verified; DMA isolation and authenticity remain
separate security claims.

The scheduler capacity is generated from the selected target's declared
isolation slots. It is not derived from filesystem enumeration limits and is
not a board-independent hardcoded slot count.

The preemption quantum and timer frequency are declared by the target profile.
The kernel does not embed a deployment-specific timing literal in scheduler
code.

Kernel-owned scheduler storage is initialized once during bootstrap. Mutable
access requires an explicit interrupt-exclusivity guarantee; uninitialized or
repeated initialization is rejected before PendSV integration.

With `abi-context-switch`, bootstrap constructs this storage from the selected
target profile. The loader registers validated application launch records,
activates the first scheduler context, and then enables the target SysTick.
The scheduler contract includes permanent retirement of a faulted context;
F405 hardware has verified that recovery resumes a ready context without
re-entering the terminated slot. F405 GDB evidence also verified that
`restore_selected` observes the slot-specific MPU code/data regions.

`prepare_pendsv` makes that sequence explicit and bounded: without a pending
request it leaves the active context unchanged; with a request it records the
outgoing state before selecting the next ready context. Register restoration and
exception return remain the responsibility of the low-level adapter.

The kernel fault boundary updates the shared runtime state atomically through
`Faulted -> Recovering -> Terminated` before entering the recovery loop. This
state channel records ownership without exposing a mutable application pointer
to exception handlers. F405 hardware has confirmed the complete transition
with the v4 invalid-PSP application fixture; this proves lifecycle accounting
and recovery entry, not automatic restart, production lifecycle policy, or
complete application isolation.

### SVC gateway

All application services use one SVC gateway. The SVC immediate is a named
gateway value reserved for Dali services; the service identifier is passed in
the stacked `r0` slot. The initial service ABI is:

| Register | Meaning |
| --- | --- |
| `r0` | service identifier on entry; status code on return |
| `r1` | first argument or argument pointer |
| `r2` | second argument |
| `r3` | third argument |
| `r12` | reserved and must be preserved by the gateway |
| `lr`, `pc`, `xPSR` | exception frame supplied by the processor |

The first service identifier is the bounded logging service. Its argument
pointer must refer only to application-readable memory, and the kernel must
validate its length and address before reading it. Unknown identifiers,
invalid pointers, invalid lengths, and malformed exception frames return a
typed failure without entering application memory or kernel state mutation.

The gateway must verify that the SVC originated in unprivileged Thread mode
with the expected PSP frame. Handler-mode calls and invalid EXC_RETURN values
are rejected. The handler returns through the original exception frame only
after the service has completed its bounded work.
