.include "defs.S"

.global main

.section .text
.global start
start:
    # Establish a stack frame at the top of memory.
    li %r3,0
    lis %sp,0x817f
    ori %sp,%sp,0xfff0
    stw %r3,0(%sp)
    mtlr %r3

    b main
    trap
