# dali target scaffold

## Purpose

Create a review scaffold for a kernel board backend from a declarative target
profile. The generated Rust file uses the .rs.template suffix deliberately:
it is not compiled until a developer reviews the HAL mapping and registers the
backend explicitly.

## Syntax

~~~text
dali target scaffold <profile> [--output <workspace-root>]
~~~

Without --output, the command searches the current directory and its parents
for the Dali workspace root. With --output, the directory must already exist.

The command creates:

- kernel/src/board/<profile>.rs.template;
- docs/boards/<profile>.md.

Existing files are never overwritten.

The generated template includes named constants for the manifest's clock,
memory, status LED, USB, optional storage, and probe metadata. It does not
invent typed HAL mappings; those remain a reviewed backend implementation step.

## Review boundary

The scaffold is not a working board backend. A developer must implement and
review typed GPIO, alternate functions, clock/reset setup, singleton
peripheral ownership, DMA/interrupt ownership, safety invariants, target
checks, and physical acceptance evidence before compiling it.
