# Feature-gated isolation ABI

The F405 isolation milestone introduces a feature-gated ABI v3 rather than
silently changing ABI v2. The implemented boundary is:

- application code runs in unprivileged Thread mode using a PSP;
- the kernel retains privileged Handler mode and MSP ownership;
- application services cross an SVC gateway with versioned service identifiers;
- applications cannot call privileged kernel functions through ordinary
  function pointers;
- MemManage, BusFault, UsageFault, and invalid exception returns are handled by
  a kernel-owned fault boundary;
- shared memory and general-purpose application lifecycle policy remain
  unsupported until their layouts, ownership, and recovery rules are
  separately specified.

The SVC frame, service identifier encoding, PSP layout, fault recovery state,
and application memory regions are specified in this document and covered by
host tests plus the recorded F405 hardware evidence below. AMRN format v4 and
v5 are package-format revisions, not ABI revisions. No separate ABI v4 or
production multi-application contract is enabled.

## Feature-gated ABI v3 boundary

ABI v3 is feature-gated and is not the default kernel execution path. ABI v2
remains the default. ABI v3 requires explicit feature selection and is still
an experimental F405 path. Its PSP/SysTick/PendSV context switching and
slot-specific MPU region switching are hardware-verified for declared
contexts, as are the processor-side fault cases and repeated invalid-PSP
recovery. General lifecycle policy, DMA isolation, and production
multi-application isolation remain open.

The source code uses the central `abi-current` selector and the version-neutral
`abi-mpu` and `abi-relocation` capabilities. These are the only ABI-related
Cargo features; version names are not repeated in feature names. A future ABI
version changes the selector and adds only the implementation-specific contract
code; ordinary kernel modules do not need a version-name replacement.
The `dali-amrn::compatibility` module is the shared ABI-family and
AMRN-format compatibility table used by the kernel-facing build contracts and
the CLI. It rejects unknown ABI versions and incompatible explicit format
requests before package construction.
The selector is enabled by `kernel/Cargo.toml`, while
`kernel/src/abi.rs` is the single source that maps the selected implementation
to its numeric package ABI version.

The repository contains a feature-gated SDK SVC call, kernel SVC frame
validator, bounded log dispatcher, MPU map, PSP transition, and kernel-owned
fault recovery behind the ABI v3 and `abi-mpu` features. These mechanisms
are available for controlled F405 testing but are not enabled by default.
