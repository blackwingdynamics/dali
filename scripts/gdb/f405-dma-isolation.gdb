# F405 DMA isolation diagnostic capture.
# Start: probe-rs gdb --chip STM32F405RGTx --protocol swd --speed 400 --reset-halt

set pagination off
set confirm off
set mem inaccessible-by-default off

define print_dma_snapshot
  printf "\n[DMA] active snapshot\n"
  printf "[DMA] LISR:   "
  x/wx 0x40026400
  printf "[DMA] CR:     "
  x/wx 0x40026458
  printf "[DMA] NDTR:   "
  x/wx 0x4002645C
  printf "[DMA] PAR:    "
  x/wx 0x40026460
  printf "[DMA] M0AR:   "
  x/wx 0x40026464
  printf "[SDIO] DCTRL: "
  x/wx 0x40012C2C
  printf "[SDIO] DCOUNT: "
  x/wx 0x40012C30
  printf "[SDIO] STA:    "
  x/wx 0x40012C34
  printf "[SDIO] FIFOCNT: "
  x/wx 0x40012C48
  printf "[TRACE] RAM snapshot: "
  x/8wx DMA_TRACE_SNAPSHOT
end

hbreak *0x0800d210
commands
  silent
  printf "\n[TRACE] DMA read window active marker\n"
  print_dma_snapshot
  printf "[TRACE] inspect active snapshot, then continue\n"
  continue
end
