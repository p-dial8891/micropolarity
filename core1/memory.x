/* memory.x - Rust Core 1 Linker Script for RP2350 */

MEMORY
{
  /* Shift Flash origin forward by 2MB (0x00200000) */
  /* Pico 2 W contains 4MB Flash total */
  FLASH : ORIGIN = 0x10200000, LENGTH = 2M

  /* Shift RAM origin forward by 64KB (0x00010000) to isolate from Core 0 */
  /* Main RAM spans from 0x20000000 to 0x20080000 (512KB) */
  RAM   : ORIGIN = 0x20010000, LENGTH = 448K

  /* Use Scratchpad Bank 1 for unmanaged cross-language sync structs */
  SHARED_RAM : ORIGIN = 0x20080000, LENGTH = 8K
}

/* Cortex-M33 Vector Table requires 256-byte alignment */
_estack = ORIGIN(RAM) + LENGTH(RAM);


SECTIONS {
    /* ### Boot ROM info
     *
     * Goes after .vector_table, to keep it in the first 4K of flash
     * where the Boot ROM (and picotool) can find it
     */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* ### Picotool 'Binary Info' Entries
     *
     * Picotool looks through this block (as we have pointers to it in our
     * header) to find interesting information.
     */
    .bi_entries : ALIGN(4)
    {
        /* We put this in the header */
        __bi_entries_start = .;
        /* Here are the entries */
        KEEP(*(.bi_entries));
        /* Keep this block a nice round size */
        . = ALIGN(4);
        /* We put this in the header */
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    /* ### Boot ROM extra info
     *
     * Goes after everything in our program, so it can contain a signature.
     */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH

} INSERT AFTER .uninit;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);


