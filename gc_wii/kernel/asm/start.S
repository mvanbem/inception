.include "defs.S"

.global main

.section .text.start
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

.section .text.change_context
.global change_context
change_context:
    mtspr SRR0,%r3
    mtspr SRR1,%r4
    rfi
