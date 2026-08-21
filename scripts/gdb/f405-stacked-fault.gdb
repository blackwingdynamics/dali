# F405 stacked-fault capture.
#
# Start probe-rs first, then in GDB run:
#   target remote :1337
#   source scripts/gdb/f405-stacked-fault.gdb
#
# The script resets the target, continues execution, and stops at the first
# Cortex-M fault. It prints the hardware-stacked frame and real fault PC.

set pagination off
set confirm off
set mem inaccessible-by-default off

define dump_stacked_fault
  printf "\n=== FAULT ===\n"
  info registers pc lr msp psp

  if (($lr & 0x4) == 0)
    set $fault_frame = $msp
  else
    set $fault_frame = $psp
  end

  printf "\n=== STACKED FRAME ===\n"
  p/x $fault_frame
  x/8wx $fault_frame

  set $fault_pc = *(unsigned int *)($fault_frame + 24)
  printf "\n=== REAL FAULT PC ===\n"
  p/x $fault_pc
  info symbol $fault_pc
  x/8i $fault_pc

  printf "\n=== SCB ===\n"
  x/wx 0xE000ED28
  x/wx 0xE000ED2C
  x/wx 0xE000ED34
  x/wx 0xE000ED38

  printf "\n=== STOPPED; DO NOT CONTINUE ===\n"
end

delete

break *BusFault
commands
  silent
  printf "\n[FAULT] BusFault handler reached\n"
  dump_stacked_fault
end

break *HardFault
commands
  silent
  printf "\n[FAULT] HardFault handler reached\n"
  dump_stacked_fault
end

break *UsageFault
commands
  silent
  printf "\n[FAULT] UsageFault handler reached\n"
  dump_stacked_fault
end

monitor reset halt
continue
