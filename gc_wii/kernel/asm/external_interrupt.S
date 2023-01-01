.include "defs.S"

.global handle_external_interrupt

.section .bss.external_interrupt_save_area
external_interrupt_save_area:
    .skip ExceptionSave_SIZE

.section .text.external_interrupt_exception_handler
.global external_interrupt_exception_handler_start
external_interrupt_exception_handler_start:
    # Save a few registers.
    mtsprg0 %r3
    mfsrr0 %r3
    mtsprg1 %r3
    mfsrr1 %r3
    mtsprg2 %r3

    # Change contexts into the second part of the handler.
    lis %r3,external_interrupt_handler_continued@h
    ori %r3,%r3,external_interrupt_handler_continued@l
    mtsrr0 %r3
    li %r3,MSR_lo_DR|MSR_lo_IR|MSR_lo_ME
    mtsrr1 %r3
    rfi
.balign 32
.global external_interrupt_exception_handler_end
external_interrupt_exception_handler_end:

.section .text.external_interrupt_handler_continued
external_interrupt_handler_continued:
    # Restore registers from the beginning of the exception handler.
    mfsprg2 %r3
    mtsrr1 %r3
    mfsprg1 %r3
    mtsrr0 %r3
    mfsprg0 %r3

    save_exception_state external_interrupt_save_area

    bl handle_external_interrupt
    # Save r3 in SPRG3. It's the return value from handle_external_interrupt and the parameter to
    # thread_scheduler.
    mtsprg3 %r3

    restore_exception_state external_interrupt_save_area

    b thread_scheduler
