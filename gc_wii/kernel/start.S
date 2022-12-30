.global main
.global bsod

# SPR indices.
DSISR = 18
DAR = 19
SRR0 = 26
SRR1 = 27
SPRG0 = 272
SPRG1 = 273
SPRG2 = 274
SPRG3 = 275
HID0 = 1008

.section .os_globals
.org 0
.global vi_interrupt_fired
vi_interrupt_fired:
    .space 1

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

.altmacro
.macro exception_handler name vector
.section .text.\name\()_exception_handler
.global \name\()_exception_handler_start
\name\()_exception_handler_start:
    # Save the registers we overwrite.
    mtspr SPRG0,%r3
    mtspr SPRG1,%r4
    mfspr %r3,SRR0
    mtspr SPRG2,%r3
    mfspr %r3,SRR1
    mtspr SPRG3,%r3

    # Craft an interrupt context for the second part of the handler and "return" to it.
    # SRR0 contains the next instruction address which is just the address of the handler.
    lis %r3,record_regs_and_bsod@h
    ori %r3,%r3,record_regs_and_bsod@l
    mtspr SRR0,%r3
    # SRR1 contains the MSR value to restore. This one enables instruction and data address
    # translation.
    li %r3,0x0030
    mtspr SRR1,%r3

    li %r3,\vector
    rfi
.balign 32
.global \name\()_exception_handler_end
\name\()_exception_handler_end:
.endm

exception_handler machine_check       0x0200
exception_handler dsi                 0x0300
exception_handler isi                 0x0400
# exception_handler external_interrupt 0x0500
exception_handler alignment           0x0600
exception_handler program             0x0700
exception_handler fp_unavailable      0x0800
# exception_handler decrementer         0x0900
exception_handler system_call         0x0c00
exception_handler trace               0x0d00
exception_handler fp_assist           0x0e00
exception_handler performance_monitor 0x0f00
exception_handler breakpoint          0x1300
exception_handler thermal_management  0x1700

.section .text.record_regs_and_bsod
record_regs_and_bsod:
    # NOTE: SPRG0/1/2/3 contain the original r3, r4, SRR0, and SRR1.

    # Fill in a BsodArgs struct at the first address following the exception handlers. That's not
    # currently allocated for anything else.
    lis %r4,0x8000
    ori %r4,%r4,0x17fc
    # Record the vector.
    stwu %r3,4(%r4)
    # Record exception handling registers.
    mfspr %r3,SPRG2  # Saved SRR0
    stwu %r3,4(%r4)
    mfspr %r3,SPRG3  # Saved SRR1
    stwu %r3,4(%r4)
    mfspr %r3,DSISR
    stwu %r3,4(%r4)
    mfspr %r3,DAR
    stwu %r3,4(%r4)
    # Record the GPRs.
    stwu %r0,4(%r4)
    stwu %r1,4(%r4)
    stwu %r2,4(%r4)
    mfspr %r3,SPRG0  # Saved r3
    stwu %r3,4(%r4)
    mfspr %r3,SPRG1  # Saved r4
    stwu %r3,4(%r4)
    stmw %r5,4(%r4) # r5..=r31; 27 words or 108 bytes
    addi %r4,%r4,108
    # Record the IBATs.
    mfspr %r3,528
    stwu %r3,4(%r4)
    mfspr %r3,529
    stwu %r3,4(%r4)
    mfspr %r3,530
    stwu %r3,4(%r4)
    mfspr %r3,531
    stwu %r3,4(%r4)
    mfspr %r3,532
    stwu %r3,4(%r4)
    mfspr %r3,533
    stwu %r3,4(%r4)
    mfspr %r3,534
    stwu %r3,4(%r4)
    mfspr %r3,535
    stwu %r3,4(%r4)
    # Record the DBATs.
    mfspr %r3,536
    stwu %r3,4(%r4)
    mfspr %r3,537
    stwu %r3,4(%r4)
    mfspr %r3,538
    stwu %r3,4(%r4)
    mfspr %r3,539
    stwu %r3,4(%r4)
    mfspr %r3,540
    stwu %r3,4(%r4)
    mfspr %r3,541
    stwu %r3,4(%r4)
    mfspr %r3,542
    stwu %r3,4(%r4)
    mfspr %r3,543
    stwu %r3,4(%r4)
    # Record the MSR.
    mfmsr %r3
    stwu %r3,4(%r4)

    # Load SRR0 for rfi into the bsod function.
    lis %r3,bsod@h
    ori %r3,%r3,bsod@l
    mtspr SRR0,%r3

    # Load SRR1 for rfi with address translation enabled.
    li %r3,0x0030
    mtspr SRR1,%r3

    # Prepare a stack frame at the top of memory.
    # TODO: Consider another place. This will clobber the base of the program stack, which might be
    # interesting to inspect. But bsod() currently uses a lot of stack space so it's not trivial to
    # place.
    lis %sp,0x817f
    ori %sp,%sp,0xfff0
    li %r3,0
    stw %r3,0(%sp)

    # Load the parameter to bsod.
    lis %r3,0x8000
    ori %r3,%r3,0x1800
    rfi

.section .text.external_interrupt_exception_handler
.global external_interrupt_exception_handler_start
external_interrupt_exception_handler_start:
    # Save registers.
    mtspr SPRG0,%r3
    mtspr SPRG1,%sp
    mfspr %r3,SRR0
    mtspr SPRG2,%r3
    mfspr %r3,SRR1
    mtspr SPRG3,%r3

    # Establish a stack frame at the top of our statically reserved exception stack range.
    # The range is physical 0x00000000..0x00000100 or effective 0x80000000..0x80000100.
    # Stack frames must be at least 16 bytes in size and must be 16-byte aligned.
    #
    # 0x00000100    First word above the stack
    # 0x000000fc +- Padding
    # 0x000000f8 |  Padding
    # 0x000000f4 |  LR save area
    # 0x000000f0 +- Bottom of the stack. Contains a null backchain pointer. SP points here.
    li %r3,0
    li %sp,0x00f0
    stw %r3,0(%sp)
    mflr %r3
    stw %r3,4(%sp)
    # Update SP for address translation
    addis %sp,%sp,0x8000

    # TODO: Call a Rust function here instead of an assembly continuation. Prepare LR so that Rust
    # function returns to an assembly cleanup function which then returns from the interrupt.

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

# NOTE: This function runs with address translation enabled.
.section .text.external_interrupt_handler_continued
external_interrupt_handler_continued:
    # Establish a stack frame.
    stwu %sp,-16(%sp)
    stw %r4,8(%sp)

    # Set a flag in OS globals.
    li %r3,1
    lis %r4,0x8000
    stb %r3,vi_interrupt_fired(%r4)

    lis %r4,0xcc00

    # Acknowledge VI interrupts 0 and 1.
    lwz %r3,0x2030(%r4)
    rlwinm %r3,%r3,0,1,31
    stw %r3,0x2030(%r4)
    lwz %r3,0x2034(%r4)
    rlwinm %r3,%r3,0,1,31
    stw %r3,0x2034(%r4)

    # Restore saved registers from the exception stack.
    lwz %r4,8(%sp)

    # Restore the final saved registers and return.
    mfspr %r3,SPRG3
    mtspr SRR1,%r3
    mfspr %r3,SPRG2
    mtspr SRR0,%r3
    mfspr %sp,SPRG1
    mfspr %r3,SPRG0
    rfi

    # TODO: Update thread scheduling; a higher priority thread may now be runnable.

.section .text.decrementer_handler
.global decrementer_exception_handler_start
decrementer_exception_handler_start:
    rfi
.balign 32
.global decrementer_exception_handler_end
decrementer_exception_handler_end:
