# Continue through the first Cortex-M fault so the kernel can persist its
# retained capture, then detach after the following reset.
#
# Usage:
#   1. Start probe-rs gdb for STM32F405RGTx.
#   2. In GDB run `target remote :1337`.
#   3. Run `source scripts/gdb/f405-persistent-capture.gdb`.
#   4. Watch the next boot on the CDC console.

set pagination off
set confirm off
set mem inaccessible-by-default off
set $fault_seen = 0

delete

break *BusFault
commands
  silent
  set $fault_seen = 1
  printf "\n[CAPTURE] BusFault trapped; continuing into kernel fault handler\n"
  continue
end

break *HardFault
commands
  silent
  set $fault_seen = 1
  printf "\n[CAPTURE] HardFault trapped; continuing into kernel fault handler\n"
  continue
end

break *UsageFault
commands
  silent
  set $fault_seen = 1
  printf "\n[CAPTURE] UsageFault trapped; continuing into kernel fault handler\n"
  continue
end

break *Reset
commands
  silent
  if $fault_seen
    printf "\n[CAPTURE] Reset after fault; detaching debugger\n"
    detach
    quit
  end
  continue
end

monitor reset halt
