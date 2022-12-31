.include "defs.S"

.global handle_external_interrupt

.section .text.external_interrupt_exception_handler
.global external_interrupt_exception_handler_start
external_interrupt_exception_handler_start:
    # Save registers. They will be restored just before returning to user code.
    mtspr SPRG0,%r3
    mtspr SPRG1,%sp
    mfspr %r3,SRR0
    mtspr SPRG2,%r3
    mfspr %r3,SRR1
    mtspr SPRG3,%r3

    # Craft an interrupt context for the second part of the handler and "return" to it.
    # SRR0 contains the next instruction address which is just the address of the handler.
    lis %r3,external_interrupt_handler_continued@h
    ori %r3,%r3,external_interrupt_handler_continued@l
    mtspr SRR0,%r3
    # SRR1 contains the MSR value to restore. This one enables machine check exceptions, indicates
    # they are recoverable (because we would rather BSOD than halt), and enables instruction and
    # data address translation.
    li %r3,0x1032
    mtspr SRR1,%r3
    rfi
.balign 32
.global external_interrupt_exception_handler_end
external_interrupt_exception_handler_end:

.section .text.external_interrupt_handler_continued
external_interrupt_handler_continued:
    # Volatile list: r0, r3-r12, lr, ctr, xer, cr0-1, cr5-7
    # There are floating-point registers to consider, and Gekko-specific things like the graphics
    # quantization registers, but kernel code doesn't use them.

    # Establish a stack frame at the top of our statically reserved exception stack range,
    # 0x80000000..0x80000100. Note that stack frames must be at least 16 bytes in size and must be
    # 16-byte aligned.
    #
    # 0x800000c0  Bottom of the stack. Contains a null backchain pointer. SP points here.
    # 0x800000c4  LR save area (used by any functions we call)
    # 0x800000c8  Saved r0
    # 0x800000cc  Saved r4
    # 0x800000d0  Saved r5
    # 0x800000d4  Saved r6
    # 0x800000d8  Saved r7
    # 0x800000dc  Saved r8
    # 0x800000e0  Saved r9
    # 0x800000e4  Saved r10
    # 0x800000e8  Saved r11
    # 0x800000ec  Saved r12
    # 0x800000f0  Saved LR
    # 0x800000f4  Saved CTR
    # 0x800000f8  Saved XER
    # 0x800000fc  Saved CR
    lis %sp,0x8000
    ori %sp,%sp,0x00c0
    li %r3,0
    stw %r3,0x00(%sp)
    stw %r0,0x08(%sp)
    stw %r4,0x0c(%sp)
    stw %r5,0x10(%sp)
    stw %r6,0x14(%sp)
    stw %r7,0x18(%sp)
    stw %r8,0x1c(%sp)
    stw %r9,0x20(%sp)
    stw %r10,0x24(%sp)
    stw %r11,0x28(%sp)
    stw %r12,0x2c(%sp)
    mflr %r3
    stw %r3,0x30(%sp)
    mfctr %r3
    stw %r3,0x34(%sp)
    mfxer %r3
    stw %r3,0x38(%sp)
    mfcr %r3
    stw %r3,0x3c(%sp)

    bl handle_external_interrupt

    # Restore saved registers from the stack.
    lwz %r0,0x08(%sp)
    lwz %r4,0x0c(%sp)
    lwz %r5,0x10(%sp)
    lwz %r6,0x14(%sp)
    lwz %r7,0x18(%sp)
    lwz %r8,0x1c(%sp)
    lwz %r9,0x20(%sp)
    lwz %r10,0x24(%sp)
    lwz %r11,0x28(%sp)
    lwz %r12,0x2c(%sp)
    lwz %r3,0x30(%sp)
    mtlr %r3
    lwz %r3,0x34(%sp)
    mtctr %r3
    lwz %r3,0x38(%sp)
    mtxer %r3
    lwz %r3,0x3c(%sp)
    mtcr %r3

    # Restore the final saved registers and return.
    mfspr %r3,SPRG3
    mtspr SRR1,%r3
    mfspr %r3,SPRG2
    mtspr SRR0,%r3
    mfspr %sp,SPRG1
    mfspr %r3,SPRG0
    rfi
