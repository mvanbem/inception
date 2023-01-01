.include "defs.S"

.global handle_system_call

.section .text.system_call_exception_handler
.global system_call_exception_handler_start
system_call_exception_handler_start:
    # Save a few registers.
    mtsprg0 %r3
    mfsrr0 %r3
    mtsprg1 %r3
    mfsrr1 %r3
    mtsprg2 %r3

    # Change contexts into the second part of the handler.
    lis %r3,system_call_handler_continued@h
    ori %r3,%r3,system_call_handler_continued@l
    mtsrr0 %r3
    li %r3,MSR_lo_DR|MSR_lo_IR|MSR_lo_ME
    mtsrr1 %r3
    rfi
.balign 32
.global system_call_exception_handler_end
system_call_exception_handler_end:

.section .text.system_call_handler_continued
system_call_handler_continued:
    # Restore registers from the beginning of the exception handler.
    mfsprg2 %r3
    mtsrr1 %r3
    mfsprg1 %r3
    mtsrr0 %r3
    mfsprg0 %r3

    save_exception_state_per_thread

    # Recover r3-5 and invoke the high-level system call handler.
    lwz %r3,ExceptionSave_r3(%r4)
    lwz %r5,ExceptionSave_r5(%r4)
    lwz %r4,ExceptionSave_r4(%r4)
    bl handle_system_call

    restore_exception_state_per_thread
    rfi
