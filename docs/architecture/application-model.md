# 7. Application model

An `.amrn` cartridge is an independently built native ARM artifact containing
an application payload. In the MVP it contains a payload linked for a
predefined SRAM execution address and a fixed entry-point ABI.

The initial lifecycle is:

```text
discovered -> header_validated -> crc32_validated -> loaded -> started
                                                        |
                                                        v
                                                   running -> failed
```

An MVP application must:

- be built for the exact reference target;
- use `no_std` and the documented ABI;
- avoid assumptions about kernel-private symbols;
- fit within the declared payload and memory limits;
- report success through a visible, deterministic action.

Native execution in a shared address space is intentionally a baseline ABI v2
limitation. ABI v3 provides a separate feature-gated processor-side boundary
with F405 hardware-verified PSP/SysTick/PendSV context switching and MPU
region switching for declared contexts. Its bounded F405 SDIO DMA policy is
separate from general DMA isolation, and complete production multi-application
lifecycle and resource isolation remain open.
