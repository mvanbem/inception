ENTRY(start)

/* The elf2dol tool will only detect BSS sections in a dedicated PHDR. */
PHDRS
{
  text PT_LOAD;
  bss PT_LOAD;
}

SECTIONS
{
  .text 0x80003100 :
  {
    *(.text .text.*);
    . = ALIGN(32);
  } :text

  .bss (ADDR(.text) + SIZEOF(.text)) :
  {
    *(.bss .bss.*);
    . = ALIGN(32);
  } :bss
}
