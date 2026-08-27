# Target tests

Hardware tests should cover:

- boot banner;
- 168 MHz clock initialization;
- board-specific storage status LED behavior;
- hardware SDIO initialization;
- one known block read;
- FAT16/FAT32 root-directory enumeration;
- `.AMRN` extension filtering and arbitrary package-name discovery;
- payload copy to the reserved SRAM address `0x20008000`;
- demo application entry;
- deterministic application LED pattern.
