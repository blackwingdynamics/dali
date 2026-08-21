# Repository-loader stack investigation for the F405 target.
#
# Source this after `target remote :1337`. The loader breakpoints stop at the
# first matching monomorphized function. Run `print_loader_stack` at each stop,
# then run `continue` to reach the next breakpoint or fault handler.

set pagination off
set confirm off
set mem inaccessible-by-default off

delete

rbreak package::load
rbreak load_binary_repository_with_contract
rbreak load_repository_package

define print_loader_stack
  printf "\n[STACK] loader breakpoint\n"
  info registers sp lr pc
  printf "[STACK] MSP alias: "
  p/x $msp
  printf "[STACK] PSP alias: "
  p/x $psp
  bt
end

break BusFault
commands
  silent
  printf "\n[STACK] BusFault reached\n"
  info registers sp lr pc
  printf "[STACK] CFSR: "
  x/wx 0xE000ED28
  printf "[STACK] BFAR: "
  x/wx 0xE000ED38
  printf "[STACK] attempted frame at $sp:\n"
  x/8wx $sp
end

break HardFault
commands
  silent
  printf "\n[STACK] HardFault reached\n"
  info registers sp lr pc
  printf "[STACK] CFSR: "
  x/wx 0xE000ED28
  printf "[STACK] HFSR: "
  x/wx 0xE000ED2C
  printf "[STACK] attempted frame at $sp:\n"
  x/8wx $sp
end

monitor reset halt
continue
