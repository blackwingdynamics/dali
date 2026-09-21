# Build and load pipeline

1. The target manifest supplies the segment alignment, capacities, permitted
   slot ranges, and ABI/format compatibility.
2. The CLI builds the application with the pinned target toolchain and a
   relocation-aware linker configuration.
3. The packaging step extracts code/data segments and the retained relocation
   records, converts only the supported records, and writes the future AMRN
   revision.
4. Host validation checks every relocation's bounds, kind, alignment, range,
   and resulting address before the cartridge is copied to storage.
5. The kernel validates the cartridge, selects the manifest-declared slot, applies relocations
   within the validated application regions, clears zero data, configures MPU
   regions, and enters the relocated entry point.
6. The kernel rejects the cartridge without copying or jumping if any relocation
   or resulting address is invalid.

The CLI now has a host-side extraction step for retained ARM ELF records. It
reads the linked .dali_code and .dali_data sections, accepts only the four
named ARM relocation kinds listed below, resolves symbol or section targets,
checks AMRN integer widths, and enforces the AMRN relocation-count limit. The
step reports the retained record count during dali app build and emits an AMRN
v3 cartridge when selected by the manifest.

The AMRN crate also contains a host-side patcher for the supported relocation
operations. It applies code/data deltas to ABS32, Thumb call, Thumb MOVW, and
Thumb MOVT patches with instruction-shape, range, alignment, and bounds checks.
The feature-gated kernel loader validates and applies the same operations from
a bounded SD stream before preparing the MPU launch frame. It selects a
manifest-declared slot by the cartridge's validated load addresses, but does not
allocate slots for concurrent applications.
