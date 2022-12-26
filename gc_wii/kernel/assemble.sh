#!/bin/bash
set -e

mkdir -p bin
rm -fv bin/start.a

powerpc-eabi-gcc -c -mogc -mcpu=750 start.S -o bin/start.o
ar rcs bin/libstart.a bin/start.o
echo "Built libstart.a"

rm -v bin/start.o
