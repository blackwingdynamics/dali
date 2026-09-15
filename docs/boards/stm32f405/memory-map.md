# STM32F405 Memory Map

The current F405 MVP memory contract is:

| Range | Purpose |
| --- | --- |
| `0x20000000`–`0x20007FFF` | Kernel reserved RAM |
| `0x20008000`–`0x20017FFF` | Application region, 64 KiB |
| `0x20018000`–`0x2001FFFF` | Kernel runtime and stack RAM |
| `0x10000000` onward | STM32F405 CCM RAM; use is backend-specific |

The current target manifest reserves the upper 64 KiB of the 512 KiB internal
Flash for the read-only artifact reader. On STM32F405, that range is inside the
preceding 128 KiB erase sector rather than being an independent erase unit.
Consequently, the current read-only layout cannot safely support in-place AMRN
writes: erasing the artifact range would also erase firmware bytes. A writable
single-cartridge layout must be resolved before implementing the F405 Flash
stager, with firmware and the physical artifact erase unit kept disjoint. The
logical cartridge limit remains 64 KiB even if the target must reserve a larger
physical erase unit.

The application load region is an F405 MVP contract, not a universal address
for future boards. Peripheral DMA buffers must remain in DMA-accessible SRAM;
CCM suitability for DMA is not assumed.

The target profile, linker script, ABI, AMRN format, and validation rules must
remain consistent when this layout changes. Processor-specific MPU register
programming belongs to the F405 backend; portable security policy belongs to
the platform-neutral layer.
