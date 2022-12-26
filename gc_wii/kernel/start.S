.global main
.global start
start:
    # Just below the top of memory. This leaves room for main() to store the return address at
    # 4(r1).
    lis %r1,0x817f
    ori %r1,%r1,0xfff8

    li %r0,0
    mtlr %r0
    b main
    trap
