MEMORY
{
    APP (rx) : ORIGIN = 0x20008000, LENGTH = 64K
}

ENTRY(amiran_entry)

SECTIONS
{
    .text : ALIGN(4)
    {
        KEEP(*(.text.amiran_entry))
        *(.text*)
        *(.rodata*)
    } > APP

    /DISCARD/ :
    {
        *(.ARM.exidx*)
        *(.ARM.extab*)
        *(.eh_frame*)
    }
}
