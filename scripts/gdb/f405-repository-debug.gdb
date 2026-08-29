# Symbol-aware F405 repository-loader fault capture.
#
# Start the probe server first:
#   probe-rs gdb --chip STM32F405RGTx --protocol swd --speed 100
#
# Start GDB with the matching DWARF-enabled ELF, then run:
#   target remote :1337
#   source scripts/gdb/f405-repository-debug.gdb
#
# The script stops at the repository-loader checkpoints and at the first
# Cortex-M fault. Do not use `next` at a fault; inspect the captured state
# while the target remains halted.

set pagination off
set confirm off
set breakpoint pending on
set disassemble-next-line on
set mem inaccessible-by-default off

define print_fault_snapshot
  printf "\n[FAULT] register snapshot\n"
  info registers r0 r1 r2 r3 r12 sp lr pc xpsr msp psp
  printf "[FAULT] CFSR:  "
  x/wx 0xE000ED28
  printf "[FAULT] HFSR:  "
  x/wx 0xE000ED2C
  printf "[FAULT] DFSR:  "
  x/wx 0xE000ED30
  printf "[FAULT] MMFAR: "
  x/wx 0xE000ED34
  printf "[FAULT] BFAR:  "
  x/wx 0xE000ED38
  printf "[FAULT] AFSR:  "
  x/wx 0xE000ED3C
  printf "[FAULT] ICSR:  "
  x/wx 0xE000ED04
  printf "[FAULT] SHCSR: "
  x/wx 0xE000ED24
  printf "[FAULT] VTOR:  "
  x/wx 0xE000ED08
  printf "[FAULT] RCC_CSR: "
  x/wx 0x40023874
  printf "[FAULT] instructions at PC:\n"
  x/12i $pc
  printf "[FAULT] stack at SP:\n"
  x/16wx $sp
  printf "[FAULT] backtrace:\n"
  bt full
end

define print_loader_checkpoint
  silent
  printf "\n[LOADER] repository checkpoint\n"
  info registers pc lr sp r0 r1 r2 r3
  bt 8
  printf "[LOADER] stopped; run `continue` to reach the next checkpoint\n"
end

delete

break kernel/src/storage/filesystem/repository.rs:175
commands
  print_loader_checkpoint
end

break kernel/src/storage/filesystem/repository.rs:189
commands
  print_loader_checkpoint
end

break kernel/src/storage/filesystem/repository.rs:203
commands
  print_loader_checkpoint
end

break HardFault
commands
  silent
  printf "\n[FAULT] HardFault reached\n"
  print_fault_snapshot
end

break BusFault
commands
  silent
  printf "\n[FAULT] BusFault reached\n"
  print_fault_snapshot
end

break UsageFault
commands
  silent
  printf "\n[FAULT] UsageFault reached\n"
  print_fault_snapshot
end

monitor reset halt
printf "[GDB] freezing IWDG while the core is halted\n"
set language c
set $dbg_apb1_fz = *(int *)0xE0042008
set {int}0xE0042008 = $dbg_apb1_fz | 0x00001000
printf "\n[GDB] Breakpoints installed; continuing from reset\n"
continue
