# SPR indices.
HID0 = 1008

MSR_lo_LE  = 0x0001
MSR_lo_RI  = 0x0002
MSR_lo_PM  = 0x0004
MSR_lo_DR  = 0x0010
MSR_lo_IR  = 0x0020
MSR_lo_IP  = 0x0040
MSR_lo_FE1 = 0x0100
MSR_lo_BE  = 0x0200
MSR_lo_SE  = 0x0400
MSR_lo_FE0 = 0x0800
MSR_lo_ME  = 0x1000
MSR_lo_FP  = 0x2000
MSR_lo_PR  = 0x4000
MSR_lo_EE  = 0x8000

MSR_hi_ILE = 0x0001
MSR_hi_POW = 0x0004

ExceptionSave_srr0 = 0x00
ExceptionSave_srr1 = 0x04
ExceptionSave_r0   = 0x08
ExceptionSave_sp   = 0x0c
ExceptionSave_r3   = 0x10
ExceptionSave_r4   = 0x14
ExceptionSave_r5   = 0x18
ExceptionSave_r6   = 0x1c
ExceptionSave_r7   = 0x20
ExceptionSave_r8   = 0x24
ExceptionSave_r9   = 0x28
ExceptionSave_r10  = 0x2c
ExceptionSave_r11  = 0x30
ExceptionSave_r12  = 0x34
ExceptionSave_ctr  = 0x38
ExceptionSave_xer  = 0x3c
ExceptionSave_lr   = 0x40
ExceptionSave_cr   = 0x44
ExceptionSave_SIZE = 0x48

.global THREAD_TABLE
.global THREAD_COUNT
.global CURRENT_THREAD

Thread_state = 0x00
Thread_waiting_for = 0x04
Thread_nia = 0x08
Thread_msr = 0x0c
Thread_ctr = 0x10
Thread_xer = 0x14
Thread_lr = 0x18
Thread_cr = 0x1c
Thread_r0 = 0x20
Thread_r1 = 0x24
Thread_r2 = 0x28
Thread_r3 = 0x2c
Thread_r4 = 0x30
Thread_r5 = 0x34
Thread_r6 = 0x38
Thread_r7 = 0x3c
Thread_r8 = 0x40
Thread_r9 = 0x44
Thread_r10 = 0x48
Thread_r11 = 0x4c
Thread_r12 = 0x50
Thread_r13 = 0x54
Thread_r14 = 0x58
Thread_r15 = 0x5c
Thread_r16 = 0x60
Thread_r17 = 0x64
Thread_r18 = 0x68
Thread_r19 = 0x6c
Thread_r20 = 0x70
Thread_r21 = 0x74
Thread_r22 = 0x78
Thread_r23 = 0x7c
Thread_r24 = 0x80
Thread_r25 = 0x84
Thread_r26 = 0x88
Thread_r27 = 0x8c
Thread_r28 = 0x90
Thread_r29 = 0x94
Thread_r30 = 0x98
Thread_r31 = 0x9c
Thread_exception_save = 0xa0
Thread_SIZE = 0xe8

ThreadState_Invalid = 0
ThreadState_Waiting = 1
ThreadState_Runnable = 2

WaitingFor_vi_di0 = 0x00000001
WaitingFor_vi_di1 = 0x00000002
WaitingFor_vi_di2 = 0x00000004
WaitingFor_vi_di3 = 0x00000008

# Saves SRR0/1, SP, and every C ABI volatile register.
.macro save_exception_state base
    # Stash r3 and r4 for a moment.
    mtsprg0 %r3
    mtsprg1 %r4

    # Load the base address.
    lis %r4,\base@h
    ori %r4,%r4,\base@l

    # Save everything.
    mfsrr0 %r3
    stw %r3,ExceptionSave_srr0(%r4)
    mfsrr1 %r3
    stw %r3,ExceptionSave_srr1(%r4)
    stw %r0,ExceptionSave_r0(%r4)
    stw %sp,ExceptionSave_sp(%r4)
    mfsprg0 %r3  # Stashed above.
    stw %r3,ExceptionSave_r3(%r4)
    mfsprg1 %r3  # Stashed above.
    stw %r3,ExceptionSave_r4(%r4)
    stw %r5,ExceptionSave_r5(%r4)
    stw %r6,ExceptionSave_r6(%r4)
    stw %r7,ExceptionSave_r7(%r4)
    stw %r8,ExceptionSave_r8(%r4)
    stw %r9,ExceptionSave_r9(%r4)
    stw %r10,ExceptionSave_r10(%r4)
    stw %r11,ExceptionSave_r11(%r4)
    stw %r12,ExceptionSave_r12(%r4)
    mfctr %r3
    stw %r3,ExceptionSave_ctr(%r4)
    mfxer %r3
    stw %r3,ExceptionSave_xer(%r4)
    mflr %r3
    stw %r3,ExceptionSave_lr(%r4)
    mfcr %r3
    stw %r3,ExceptionSave_cr(%r4)
.endm

# Restores SRR0/1, SP, and every C ABI volatile register.
.macro restore_exception_state base
    # Load the base address.
    lis %r4,\base@h
    ori %r4,%r4,\base@l

    # Restore everything.
    lwz %r3,ExceptionSave_cr(%r4)
    mtcr %r3
    lwz %r3,ExceptionSave_lr(%r4)
    mtlr %r3
    lwz %r3,ExceptionSave_xer(%r4)
    mtxer %r3
    lwz %r3,ExceptionSave_ctr(%r4)
    mtctr %r3
    lwz %r12,ExceptionSave_r12(%r4)
    lwz %r11,ExceptionSave_r11(%r4)
    lwz %r10,ExceptionSave_r10(%r4)
    lwz %r9,ExceptionSave_r9(%r4)
    lwz %r8,ExceptionSave_r8(%r4)
    lwz %r7,ExceptionSave_r7(%r4)
    lwz %r6,ExceptionSave_r6(%r4)
    lwz %r5,ExceptionSave_r5(%r4)
    # Skip r3 and r4 for the moment. They are needed for moves.
    lwz %sp,ExceptionSave_sp(%r4)
    lwz %r0,ExceptionSave_r0(%r4)
    lwz %r3,ExceptionSave_srr1(%r4)
    mtsrr1 %r3
    lwz %r3,ExceptionSave_srr0(%r4)
    mtsrr0 %r3

    # Finally, restore r3 and r4.
    lwz %r3,ExceptionSave_r3(%r4)
    lwz %r4,ExceptionSave_r4(%r4)
.endm

# Saves SRR0/1, SP, and every C ABI volatile register.
.macro save_exception_state_per_thread
    # Stash r3 and r4 for a moment.
    mtsprg0 %r3
    mtsprg1 %r4

    # Load the base address.
    lis %r4,CURRENT_THREAD@ha
    lwz %r4,CURRENT_THREAD@l(%r4)
    mulli %r4,%r4,Thread_SIZE
    addis %r4,%r4,(THREAD_TABLE + Thread_exception_save)@ha
    addi %r4,%r4,(THREAD_TABLE + Thread_exception_save)@l

    # Save everything.
    mfsrr0 %r3
    stw %r3,ExceptionSave_srr0(%r4)
    mfsrr1 %r3
    stw %r3,ExceptionSave_srr1(%r4)
    stw %r0,ExceptionSave_r0(%r4)
    stw %sp,ExceptionSave_sp(%r4)
    mfsprg0 %r3  # Stashed above.
    stw %r3,ExceptionSave_r3(%r4)
    mfsprg1 %r3  # Stashed above.
    stw %r3,ExceptionSave_r4(%r4)
    stw %r5,ExceptionSave_r5(%r4)
    stw %r6,ExceptionSave_r6(%r4)
    stw %r7,ExceptionSave_r7(%r4)
    stw %r8,ExceptionSave_r8(%r4)
    stw %r9,ExceptionSave_r9(%r4)
    stw %r10,ExceptionSave_r10(%r4)
    stw %r11,ExceptionSave_r11(%r4)
    stw %r12,ExceptionSave_r12(%r4)
    mfctr %r3
    stw %r3,ExceptionSave_ctr(%r4)
    mfxer %r3
    stw %r3,ExceptionSave_xer(%r4)
    mflr %r3
    stw %r3,ExceptionSave_lr(%r4)
    mfcr %r3
    stw %r3,ExceptionSave_cr(%r4)
.endm

# Restores SRR0/1, SP, and every C ABI volatile register.
.macro restore_exception_state_per_thread
    # Load the base address.
    lis %r4,CURRENT_THREAD@ha
    lwz %r4,CURRENT_THREAD@l(%r4)
    mulli %r4,%r4,Thread_SIZE
    addis %r4,%r4,(THREAD_TABLE + Thread_exception_save)@ha
    addi %r4,%r4,(THREAD_TABLE + Thread_exception_save)@l

    # Restore everything.
    lwz %r3,ExceptionSave_cr(%r4)
    mtcr %r3
    lwz %r3,ExceptionSave_lr(%r4)
    mtlr %r3
    lwz %r3,ExceptionSave_xer(%r4)
    mtxer %r3
    lwz %r3,ExceptionSave_ctr(%r4)
    mtctr %r3
    lwz %r12,ExceptionSave_r12(%r4)
    lwz %r11,ExceptionSave_r11(%r4)
    lwz %r10,ExceptionSave_r10(%r4)
    lwz %r9,ExceptionSave_r9(%r4)
    lwz %r8,ExceptionSave_r8(%r4)
    lwz %r7,ExceptionSave_r7(%r4)
    lwz %r6,ExceptionSave_r6(%r4)
    lwz %r5,ExceptionSave_r5(%r4)
    # Skip r3 and r4 for the moment. They are needed for moves.
    lwz %sp,ExceptionSave_sp(%r4)
    lwz %r0,ExceptionSave_r0(%r4)
    lwz %r3,ExceptionSave_srr1(%r4)
    mtsrr1 %r3
    lwz %r3,ExceptionSave_srr0(%r4)
    mtsrr0 %r3

    # Finally, restore r3 and r4.
    lwz %r3,ExceptionSave_r3(%r4)
    lwz %r4,ExceptionSave_r4(%r4)
.endm
