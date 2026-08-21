# F405 repository-loader capture session.
#
# Start probe-rs first:
#   sudo probe-rs gdb --chip STM32F405RGTx --protocol swd --speed 100
#
# Then in GDB:
#   target remote :1337
#   source scripts/gdb/f405-repository-capture.gdb
#   continue
#
# This script captures the first reset, storage initialization, repository
# loader entry, and any Cortex-M fault without requiring debug information.

set pagination off
set confirm off
set mem inaccessible-by-default off

define print_fault_registers
  printf "\n[FAULT] registers\n"
  info registers r0 r1 r2 r3 r12 sp lr pc xpsr msp psp
  printf "[FAULT] CFSR: "
  x/wx 0xE000ED28
  printf "[FAULT] HFSR: "
  x/wx 0xE000ED2C
  printf "[FAULT] MMFAR: "
  x/wx 0xE000ED34
  printf "[FAULT] BFAR:  "
  x/wx 0xE000ED38
  printf "[FAULT] SHCSR: "
  x/wx 0xE000ED24
  printf "[FAULT] RCC_CSR: "
  x/wx 0x40023874
  printf "[FAULT] instructions:\n"
  x/8i $pc
  printf "[FAULT] stack:\n"
  x/8wx $sp
end

define capture_fault
  silent
  printf "\n[FAULT] %s\n", $arg0
  print_fault_registers
  printf "[FAULT] execution stopped; inspect the snapshot before continuing\n"
end

define print_sdio_snapshot
  printf "\n[SDIO] register snapshot\n"
  printf "[SDIO] DTIMER: "
  x/wx 0x40012c24
  printf "[SDIO] DCTRL:  "
  x/wx 0x40012c2c
  printf "[SDIO] DCOUNT: "
  x/wx 0x40012c30
  printf "[SDIO] STA:    "
  x/wx 0x40012c34
  printf "[DMA] LISR:   "
  x/wx 0x40026400
  printf "[DMA] HISR:   "
  x/wx 0x40026404
  printf "[DMA] CR:     "
  x/wx 0x40026458
  printf "[DMA] NDTR:   "
  x/wx 0x4002645c
  printf "[DMA] PAR:    "
  x/wx 0x40026460
  printf "[DMA] M0AR:   "
  x/wx 0x40026464
end

delete

break *Reset
commands
  silent
  printf "\n[BOOT] Reset reached\n"
  printf "[BOOT] RCC_CSR: "
  x/wx 0x40023874
  continue
end

break *BusFault
commands
  capture_fault "BusFault reached"
end

break *HardFault
commands
  capture_fault "HardFault reached"
end

break *UsageFault
commands
  capture_fault "UsageFault reached"
end

break *DefaultHandler
commands
  capture_fault "DefaultHandler reached"
end

break dali_kernel::bootstrap::storage::initialization::initialize
commands
  silent
  printf "\n[TRACE] storage::initialize entered\n"
  info registers pc lr sp
  continue
end

rbreak .*RawSdioReader.*read_block.*
commands
  silent
  printf "\n[TRACE] RawSdioReader::read_block entered\n"
  info registers pc lr sp r0 r1 r2 r3
  print_sdio_snapshot
  bt
end

rbreak .*Sdio.*SdCard.*init.*
commands
  silent
  printf "\n[TRACE] stm32f4xx-hal SdCard::init entered\n"
  info registers pc lr sp r0 r1 r2 r3
  print_sdio_snapshot
  bt
end

monitor reset halt
