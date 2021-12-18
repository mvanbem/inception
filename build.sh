#!/bin/bash
set -e

function subcommand_build {
    rm -rf build/*

    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack --release -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack_map \
        --dst ../build \
        "$@"

    popd >/dev/null


    echo
    echo === Building bsp-loader-gx ===
    pushd gc_wii >/dev/null

    cargo build -p bsp-loader-gx --release --no-default-features --features=gamecube
    elf2dol \
        target/powerpc-none-eabi/release/bsp-loader-gx \
        ../build/bsp-loader-gx_gamecube.dol

    cargo build -p bsp-loader-gx --release --no-default-features --features=wii
    elf2dol \
        target/powerpc-none-eabi/release/bsp-loader-gx \
        ../build/bsp-loader-gx_wii.dol

    popd >/dev/null


    echo === SUCCESS ===
}

function subcommand_other {
    pushd pc >/dev/null
    cargo run -p inception-pack --release -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        "$@"
}

case $1 in
    "" | build) subcommand_build;;
    *) subcommand_other "$@";;
    *)
        echo "unknown subcommand: $1" >&2
        exit 1
esac
