# F405 exception capture session for repository-loader failures.
#
# Usage:
#   1. Start `probe-rs gdb` in another terminal.
#   2. In GDB, run `target remote :1337`.
#   3. Run `source scripts/gdb/f405-exception-capture.gdb`.
#
# The script uses ELF symbols instead of fixed firmware addresses, so it also
# works after a rebuild changes the exception-handler locations.

set pagination off
set confirm off
set mem inaccessible-by-default off

define print_fault_snapshot
  printf "\n[FAULT] register snapshot\n"
  info registers r0 r1 r2 r3 r12 sp lr pc xpsr msp psp
  printf "[FAULT] CFSR:  "
  x/wx 0xE000ED28
  printf "[FAULT] HFSR:  "
  x/wx 0xE000ED2C
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
  x/8i $pc
  printf "[FAULT] memory at SP:\n"
  x/8wx $sp
end

delete

break BusFault
commands
  silent
  printf "\n[FAULT] BusFault reached\n"
  print_fault_snapshot
end

break HardFault
commands
  silent
  printf "\n[FAULT] HardFault reached\n"
  print_fault_snapshot
end

break UsageFault
commands
  silent
  printf "\n[FAULT] UsageFault reached\n"
  print_fault_snapshot
end

break DefaultHandler
commands
  silent
  printf "\n[FAULT] DefaultHandler reached\n"
  print_fault_snapshot
end

rbreak pet_repository_chunk
commands
  silent
  printf "\n[WATCHDOG] repository chunk pet\n"
  info registers pc lr sp
  continue
end

break Reset
commands
  silent
  printf "\n[BOOT] Reset reached\n"
  printf "[BOOT] RCC_CSR: "
  x/wx 0x40023874
end

monitor reset halt
continue
