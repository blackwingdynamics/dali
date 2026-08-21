# F405 fault-capture session for repository-loader failures.
#
# Source this after `target remote :1337`. The script resets the target,
# continues execution, and stops at the first BusFault or HardFault. At the
# exception entry point `$sp` addresses the hardware-stacked frame:
# r0, r1, r2, r3, r12, lr, pc, xpsr.

set pagination off
set confirm off
set mem inaccessible-by-default off

delete
break BusFault
commands
  silent
  printf "\n[FAULT] BusFault entry\n"
  info registers r0 r1 r2 r3 r12 sp lr pc xpsr
  printf "[FAULT] CFSR:  "
  x/wx 0xE000ED28
  printf "[FAULT] HFSR:  "
  x/wx 0xE000ED2C
  printf "[FAULT] BFAR:  "
  x/wx 0xE000ED38
  printf "[FAULT] AFSR:  "
  x/wx 0xE000ED3C
  printf "[FAULT] stacked frame at $sp (r0 r1 r2 r3 r12 lr pc xpsr):\n"
  x/8wx $sp
end

break HardFault
commands
  silent
  printf "\n[FAULT] HardFault entry\n"
  info registers r0 r1 r2 r3 r12 sp lr pc xpsr
  printf "[FAULT] CFSR:  "
  x/wx 0xE000ED28
  printf "[FAULT] HFSR:  "
  x/wx 0xE000ED2C
  printf "[FAULT] BFAR:  "
  x/wx 0xE000ED38
  printf "[FAULT] AFSR:  "
  x/wx 0xE000ED3C
  printf "[FAULT] stacked frame at $sp (r0 r1 r2 r3 r12 lr pc xpsr):\n"
  x/8wx $sp
end

monitor reset halt
continue
