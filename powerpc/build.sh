#!/bin/bash
set -e

cargo build --release --no-default-features --features=gamecube
elf2dol \
    target/powerpc-none-eabi/release/bsp-loader-gx \
    target/powerpc-none-eabi/release/bsp-loader-gx_gamecube.dol \

cargo build --release --no-default-features --features=wii
elf2dol \
    target/powerpc-none-eabi/release/bsp-loader-gx \
    target/powerpc-none-eabi/release/bsp-loader-gx_wii.dol \
