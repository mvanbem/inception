ENTRY(start)

/* The elf2dol tool will only detect BSS sections in a dedicated PHDR. */
PHDRS
{
  text PT_LOAD;
  rodata PT_LOAD;
  bss PT_LOAD;
  discarded PT_LOAD;
}

SECTIONS
{
  .os_globals 0x00000000 (NOLOAD) :
  {
    *(.os_globals);
  } :discarded

  .text 0x80003100 :
  {
    *(.text .text.*);
    . = ALIGN(32);
  } :text

  .rodata (ADDR(.text) + SIZEOF(.text)) :
  {
    *(.rodata .rodata.*);
    . = ALIGN(32);
  } :rodata

  .bss (ADDR(.rodata) + SIZEOF(.rodata)) :
  {
    *(.bss .bss.*);
    . = ALIGN(32);
  } :bss
}
