# Multi-slot isolation layout

The following layout is active in the F405 target manifest and is generated
from target metadata rather than embedded in kernel policy:

```text
0x20000000 - 0x20007FFF   Kernel DMA and transport buffers, 32 KiB
0x20008000 - 0x2000BFFF   Application slot 0 code, 16 KiB
0x2000C000 - 0x2000FFFF   Application slot 0 data/PSP, 16 KiB
0x20010000 - 0x20013FFF   Application slot 1 code, 16 KiB
0x20014000 - 0x20017FFF   Application slot 1 data/PSP, 16 KiB
0x20018000 - 0x2001FFFF   Runtime-region repository workspace, 32 KiB
0x10000000 - 0x1000FFFF   Kernel runtime, static state, and privileged stack
```

The kernel runtime migration must prove that every DMA-visible buffer remains
in the first SRAM region; CCM is not DMA-accessible on this MCU. The repository
workspace is a bounded, privileged BSS allocation selected from the target
manifest's runtime region. It is not application memory and is not implicitly
shared with unprivileged applications. Any future shared-memory policy must
assign explicit MPU permissions and ownership.
