MEMORY
{
  /* Internal Flash memory for the kernel. */
  FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 512K

  /* Kernel reserved RAM. The application region begins at 0x20008000. */
  RAM (xrw)   : ORIGIN = 0x20000000, LENGTH = 32K
}
