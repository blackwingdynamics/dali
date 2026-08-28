# Active safety boundaries

- [x] SDIO implementation and storage behavior remain at the Known-Good
  baseline; no changes are permitted in `kernel/src/platform/f405/sdio_raw/`,
  `kernel/src/storage/`, or SDIO manifest limits during these phases.
- [x] USB CDC core servicing and polling loops are frozen.
- [x] ILI9341 and all SPI display experiments are frozen.

## Evidence rules

Completion requires implementation, relevant host/target validation, and
hardware evidence when the behavior depends on a physical target. The
canonical records are [`../testing/README.md`](../testing/README.md),
[`../mvp-acceptance/README.md`](../mvp-acceptance/README.md), and
[`../security/README.md`](../security/README.md).

The completed security claims are scoped to the configured STM32F405 path.
They do not claim arbitrary DMA-controller isolation, confidentiality,
complete production multi-application isolation, or pre-reset Secure Boot.
The feature-gated ABI v3 context-switching and MPU-switching path remains an
experimental platform capability, while the baseline ABI v2 application
remains trusted native code.

## Working rule

Keep each phase atomic, preserve hardware-agnostic contracts, and obtain
explicit approval before changing architecture, boot path, storage layout,
loader mode, ABI, or package layout.
