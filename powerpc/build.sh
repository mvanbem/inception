#!/bin/bash
set -e

cargo build --release
elf2dol target/powerpc-unknown-eabi/release/bsp-loader-gx{,.dol}
