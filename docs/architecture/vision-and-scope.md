# 1. Identity and vision

Dali OS is a small, modular, real-time-oriented embedded platform written in Rust for STM32 microcontrollers and, eventually, industrial machines and autonomous vehicles.

The name comes from Dali, the Georgian hunting goddess who could take the form of a bird or animal. The application cartridge format is named `.amrn` — Amiran Native — after Amirani, Dali's son. The project uses this mythology as part of its identity while keeping the technical platform understandable to an international audience.

The central architectural idea is:

> The kernel is the stable platform. Applications are independently built native payloads delivered in cartridges and loaded by the kernel.

## 2. Scope

### Long-term goals

- `no_std` Rust kernel for a focused STM32 platform;
- predictable scheduling and bounded real-time services;
- hardware services for sensors, motors, communication, storage, and power;
- independently built and deployed applications;
- a Rust SDK (`dali`);
- a cartridge and device-management CLI (`dali` command, `dali-cli` cartridge);
- signed cartridges, compatibility checks, rollback, and safe recovery.

### Explicit non-goals for the first version

- desktop or general-purpose operating-system features;
- Linux compatibility;
- a full MMU-based process model;
- support for every STM32 family at once;
- C ABI compatibility;
- claiming memory isolation before an MPU or another isolation mechanism is implemented.
