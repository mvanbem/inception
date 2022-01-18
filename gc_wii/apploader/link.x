OUTPUT_FORMAT(binary)

MEMORY
{
  CODE (rwx) : ORIGIN = 0x81200000, LENGTH = 8K
}

SECTIONS
{
  .code :
  AT(0)
  {
    KEEP(*(.apploader.entry));
    /* TODO: Eliminate some unnecessary sections. */
    *(*);
    . = LENGTH(CODE);
  } > CODE = 0
}
