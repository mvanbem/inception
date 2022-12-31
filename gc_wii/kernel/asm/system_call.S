.include "defs.S"

.global handle_system_call

.section .text.system_call_exception_handler
.global system_call_exception_handler_start
system_call_exception_handler_start:
    # Save registers. They will be restored just before returning to user code.
    mfspr %r14,SRR0
    mfspr %r15,SRR1

    # Craft an interrupt context for the second part of the handler and "return" to it.
    # SRR0 contains the next instruction address which is just the address of the handler.
    lis %r6,system_call_handler_continued@h
    ori %r6,%r6,system_call_handler_continued@l
    mtspr SRR0,%r6
    # SRR1 contains the MSR value to restore. This one enables machine check exceptions, enables
    # instruction and data address translation, and sets the exception recoverable bit because the
    # original SRR0/1 have been saved.
    li %r6,0x1032
    mtspr SRR1,%r6
    rfi

.balign 32
.global system_call_exception_handler_end
system_call_exception_handler_end:

.section .text.system_call_handler_continued
system_call_handler_continued:
    # Establish a stack frame at the top of our statically reserved system call stack range,
    # 0x80800000..0x80800100. That's at the top of the first 8 MiB of physical memory.
    #
    # 0x808000d0  Bottom of the stack. Contains a null backchain pointer. SP points here.
    # 0x808000d4  LR save area (used by any functions we call)
    # 0x808000d8  Saved SP
    # 0x808000dc  Saved r2
    # 0x808000e0  Saved r13
    # 0x808000e4  Saved r29
    # 0x808000e8  Saved r30
    # 0x808000ec  Saved r31
    # 0x808000f0  Saved LR
    # 0x808000f4  Saved CTR
    # 0x808000f8  Saved XER
    # 0x808000fc  Saved CR
    mr %r7,%sp  # Save the old stack pointer for a moment.
    lis %sp,0x8080
    ori %sp,%sp,0x00d0
    li %r6,0
    stw %r6,0x00(%sp)
    stw %r7,0x08(%sp)  # This is the SP from a moment ago.
    stw %r2,0x0c(%sp)
    stw %r13,0x10(%sp)
    stw %r29,0x14(%sp)
    stw %r30,0x18(%sp)
    stw %r31,0x1c(%sp)
    mflr %r6
    stw %r6,0x20(%sp)
    mfctr %r6
    stw %r6,0x24(%sp)
    mfxer %r6
    stw %r6,0x28(%sp)
    mfcr %r6
    stw %r6,0x2c(%sp)

    bl handle_system_call

    # Restore saved registers from the stack.
    lwz %r6,0x2c(%sp)
    mtcr %r6
    lwz %r6,0x28(%sp)
    mtxer %r6
    lwz %r6,0x24(%sp)
    mtctr %r6
    lwz %r6,0x20(%sp)
    mtlr %r6
    lwz %r31,0x1c(%sp)
    lwz %r30,0x18(%sp)
    lwz %r29,0x14(%sp)
    lwz %r13,0x10(%sp)
    lwz %r2,0x0c(%sp)
    lwz %sp,0x08(%sp)

    # Restore the final saved registers and return.
    mtspr SRR0,%r14
    mtspr SRR1,%r15
    rfi
