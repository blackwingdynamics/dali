# Test-only GDB setup for the F405 BusFault decoder fixture.
# This changes live MPU state through SWD; it is never linked into the kernel.

set pagination off
set confirm off
set mem inaccessible-by-default off
set language c

set $scb_cfsr = 0xE000ED28
set $mpu_ctrl = 0xE000ED94

# Disable the MPU only for this decoder test so the reserved code-region access
# reaches the bus fabric instead of being rejected as MemManage.
set {int} $scb_cfsr = 0xFFFFFFFF
set {int} $mpu_ctrl = 0

printf "Test-only MPU disable active for BusFault decoding.\n"
printf "Continue and stop at handle_with_frame to inspect the BusFault record.\n"
