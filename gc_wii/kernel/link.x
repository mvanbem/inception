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
    *(*);
    . = ALIGN(32);
  } :text

  /* .bss 0x80000000 :
  {
    . = ABSOLUTE(ADDR(.text));
  } :bss */

  /* .bss2 (ADDR(.text) + SIZEOF(.text)) :
  {
    *(.bss .bss.*);
    . = ABSOLUTE(0x80400000);
  } :bss */
}
