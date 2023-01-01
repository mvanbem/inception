.include "defs.S"

# Inputs:
# - r3: pointer to thread struct
# - SPRG0: thread LR
# - SPRG1: thread r3
# - SRR0: thread NIA
# - SRR1: thread MSR
.global save_thread_state
save_thread_state:
    # Store r4 first so we can use it to move things around.
    stw %r4,Thread_r4(%r3)
    # Store execution state.
    mfsrr0 %r4
    stw %r4,Thread_nia(%r3)
    mfsrr1 %r4
    stw %r4,Thread_msr(%r3)
    # Store SPRs.
    mfctr %r4
    stw %r4,Thread_ctr(%r3)
    mfxer %r4
    stw %r4,Thread_xer(%r3)
    mfsprg0 %r4  # LR is provided in SPRG0.
    stw %r4,Thread_lr(%r3)
    mfcr %r4
    stw %r4,Thread_cr(%r3)
    # Store GPRs.
    stw %r0,Thread_r0(%r3)
    stw %r1,Thread_r1(%r3)
    stw %r2,Thread_r2(%r3)
    mfsprg1 %r4  # r3 is provided in SPRG1.
    stw %r4,Thread_r3(%r3)
    # r4 was already stored.
    stw %r5,Thread_r5(%r3)
    stw %r6,Thread_r6(%r3)
    stw %r7,Thread_r7(%r3)
    stw %r8,Thread_r8(%r3)
    stw %r9,Thread_r9(%r3)
    stw %r10,Thread_r10(%r3)
    stw %r11,Thread_r11(%r3)
    stw %r12,Thread_r12(%r3)
    stw %r13,Thread_r13(%r3)
    stw %r14,Thread_r14(%r3)
    stw %r15,Thread_r15(%r3)
    stw %r16,Thread_r16(%r3)
    stw %r17,Thread_r17(%r3)
    stw %r18,Thread_r18(%r3)
    stw %r19,Thread_r19(%r3)
    stw %r20,Thread_r20(%r3)
    stw %r21,Thread_r21(%r3)
    stw %r22,Thread_r22(%r3)
    stw %r23,Thread_r23(%r3)
    stw %r24,Thread_r24(%r3)
    stw %r25,Thread_r25(%r3)
    stw %r26,Thread_r26(%r3)
    stw %r27,Thread_r27(%r3)
    stw %r28,Thread_r28(%r3)
    stw %r29,Thread_r29(%r3)
    stw %r30,Thread_r30(%r3)
    stw %r31,Thread_r31(%r3)
    blr

# Inputs:
# - r3: pointer to thread struct
#
# Does not return.
.global restore_thread_state
restore_thread_state:
    # Restore execution state.
    lwz %r4,Thread_nia(%r3)
    mtsrr0 %r4
    lwz %r4,Thread_msr(%r3)
    mtsrr1 %r4
    # Restore SPRs.
    lwz %r4,Thread_ctr(%r3)
    mtctr %r4
    lwz %r4,Thread_xer(%r3)
    mtxer %r4
    lwz %r4,Thread_lr(%r3)
    mtlr %r4
    lwz %r4,Thread_cr(%r3)
    mtcr %r4
    # Restore GPRs.
    lwz %r0,Thread_r0(%r3)
    lwz %r1,Thread_r1(%r3)
    lwz %r2,Thread_r2(%r3)
    # r3 is restored last.
    lwz %r4,Thread_r4(%r3)
    lwz %r5,Thread_r5(%r3)
    lwz %r6,Thread_r6(%r3)
    lwz %r7,Thread_r7(%r3)
    lwz %r8,Thread_r8(%r3)
    lwz %r9,Thread_r9(%r3)
    lwz %r10,Thread_r10(%r3)
    lwz %r11,Thread_r11(%r3)
    lwz %r12,Thread_r12(%r3)
    lwz %r13,Thread_r13(%r3)
    lwz %r14,Thread_r14(%r3)
    lwz %r15,Thread_r15(%r3)
    lwz %r16,Thread_r16(%r3)
    lwz %r17,Thread_r17(%r3)
    lwz %r18,Thread_r18(%r3)
    lwz %r19,Thread_r19(%r3)
    lwz %r20,Thread_r20(%r3)
    lwz %r21,Thread_r21(%r3)
    lwz %r22,Thread_r22(%r3)
    lwz %r23,Thread_r23(%r3)
    lwz %r24,Thread_r24(%r3)
    lwz %r25,Thread_r25(%r3)
    lwz %r26,Thread_r26(%r3)
    lwz %r27,Thread_r27(%r3)
    lwz %r28,Thread_r28(%r3)
    lwz %r29,Thread_r29(%r3)
    lwz %r30,Thread_r30(%r3)
    lwz %r31,Thread_r31(%r3)
    # And finally, restore r3.
    lwz %r3,Thread_r3(%r3)
    rfi

# Restores a runnable thread if one exists, otherwise goes to sleep. This thread returns from this
# function the next time it is run.
.global call_thread_scheduler
call_thread_scheduler:
    # SRR0/1: Set as though we raised an exception just before the end of this function.
    lis %r3,1f@h
    ori %r3,%r3,1f@l
    mtsrr0 %r3
    li %r3,MSR_lo_DR|MSR_lo_IR
    mtsrr1 %r3
    # SPRG3: No interrupts observed.
    li %r3,0
    mtsprg3 %r3
    b thread_scheduler
1:
    blr

# Restores a runnable thread if one exists, otherwise goes to sleep.
#
# Inputs:
# - SPRG3: An interrupt bitfield with ones for any interrupts that have just occurred.
.global thread_scheduler
thread_scheduler:
    #
    # Suspend the current thread
    #

    # Stash r3 to free it up.
    mtsprg1 %r3
    # Load the current thread ID.
    lis %r3,CURRENT_THREAD@ha
    lwz %r3,CURRENT_THREAD@l(%r3)
    # Skip suspending if we weren't running a thread.
    cmpwi %r3,-1
    beq 1f

    # SPRG1 already contains thread r3
    # SPRG0: thread LR
    mflr %r3
    mtsprg0 %r3
    # r3: pointer to current thread
    lis %r3,CURRENT_THREAD@h
    ori %r3,%r3,CURRENT_THREAD@l
    lwz %r3,0(%r3)
    mulli %r3,%r3,Thread_SIZE
    addis %r3,%r3,THREAD_TABLE@ha
    addi %r3,%r3,THREAD_TABLE@l
    bl save_thread_state
1:

    # All registers are now available.

    # r6 points to the current thread ID.
    lis %r6,CURRENT_THREAD@h
    ori %r6,%r6,CURRENT_THREAD@l

    # r7 points to the current thread count.
    lis %r7,THREAD_COUNT@h
    ori %r7,%r7,THREAD_COUNT@l

    #
    # Scan the thread table, updating waiting_for and runnable states.
    #

    # Load the thread count as a loop counter.
    lwz %r3,0(%r7)
    mtctr %r3

    # r4 points to the thread under consideration.
    lis %r4,THREAD_TABLE@h
    ori %r4,%r4,THREAD_TABLE@l

    # r8 contains the interrupt mask.
    mfsprg3 %r8

    b update_runnable_loop_body
update_runnable_loop:
    addi %r4,%r4,Thread_SIZE
update_runnable_loop_body:
    # Skip threads that are not waiting.
    lbz %r3,Thread_state(%r4)
    cmplwi %r3,ThreadState_Waiting
    bne 1f

    # Skip threads that are not waiting for any of the interrupts that just fired.
    lwz %r3,Thread_waiting_for(%r4)
    and. %r3,%r3,%r8
    beq 1f

    # Mark the thread as runnable.
    li %r3,ThreadState_Runnable
    stb %r3,Thread_state(%r4)
1:
    bdnz update_runnable_loop

    #
    # Scan the thread table and resume the first runnable thread, if there is one.
    #

    # Load the thread count as a loop counter.
    lwz %r3,0(%r7)
    mtctr %r3

    # r4 tracks the thread ID under consideration.
    li %r4,0

    # r5 points to the thread under consideration.
    lis %r5,THREAD_TABLE@h
    ori %r5,%r5,THREAD_TABLE@l

    b find_runnable_loop_body
find_runnable_loop:
    addi %r4,%r4,1
    addi %r5,%r5,Thread_SIZE
find_runnable_loop_body:
    # Skip threads that are not runnable.
    lbz %r3,Thread_state(%r5)
    cmplwi %r3,ThreadState_Runnable
    bne 1f

    # Update the current thread ID.
    stw %r4,0(%r6)

    # Run the runnable thread.
    mr %r3,%r5  # r3: pointer to thread struct
    b restore_thread_state
1:
    bdnz find_runnable_loop

    #
    # No threads are runnable. Wait for an interrupt.
    #

    # Update the current thread ID.
    li %r3,-1
    stw %r3,0(%r6)

    # Enable external interrupts.
    mfmsr %r3
    ori %r3,%r3,MSR_lo_EE
    mtmsr %r3
1:
    b 1b
