# STM32F405 Memory Map

The reference board uses an STM32F405RGT6 with up to 1 MiB of internal Flash,
up to 192 KiB of SRAM, and 4 KiB of backup SRAM. The values below describe the
current Dali target profile, not the complete physical memory capacity.

The current F405 MVP memory contract is:

| Range | Purpose |
| --- | --- |
| `0x20000000`–`0x20007FFF` | Kernel reserved RAM |
| `0x20008000`–`0x20017FFF` | Application region, 64 KiB |
| `0x20018000`–`0x2001FFFF` | Kernel runtime and stack RAM |
| `0x10000000` onward | STM32F405 CCM RAM; use is backend-specific |

The target manifest currently exposes a 512 KiB Flash window: 384 KiB for the
firmware image and the final 128 KiB internal Flash erase sector for the
artifact store. The logical cartridge capacity within that physical region is
limited to 64 KiB. The remaining physical Flash above the current profile is
not yet part of the Dali layout.

The artifact sector is reserved for cartridge storage. Two 64 KiB cartridge
slots do not fit together with publication and recovery metadata in the same
128 KiB erase sector; a future two-slot design must allocate additional unused
Flash sectors and update the target manifest, linker, writer, and recovery
contracts together.

The application load region is an F405 MVP contract, not a universal address
for future boards. Peripheral DMA buffers must remain in DMA-accessible SRAM;
CCM suitability for DMA is not assumed.

The target profile, linker script, ABI, AMRN format, and validation rules must
remain consistent when this layout changes. Processor-specific MPU register
programming belongs to the F405 backend; portable security policy belongs to
the platform-neutral layer.
