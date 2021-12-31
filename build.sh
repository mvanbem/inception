#!/bin/bash
set -e

function subcommand_build {
    rm -rf build/*

    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack_map \
        --dst ../build \
        "$@"

    popd >/dev/null


    echo
    echo === Building bsp-loader-gx ===
    pushd gc_wii >/dev/null

    cargo build -p bsp-loader-gx $release_flag --no-default-features --features=gamecube
    elf2dol \
        target/powerpc-none-eabi/release/bsp-loader-gx \
        ../build/bsp-loader-gx_gamecube.dol

    cargo build -p bsp-loader-gx $release_flag --no-default-features --features=wii
    elf2dol \
        target/powerpc-none-eabi/release/bsp-loader-gx \
        ../build/bsp-loader-gx_wii.dol

    popd >/dev/null


    echo === SUCCESS ===
}

function subcommand_other {
    pushd pc >/dev/null
    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        "$@"
}

release_flag=--release
while true; do
    case $1 in
        --debug)
            release_flag=
            shift
            ;;
        "" | build)
            subcommand_build
            exit 0
            ;;
        *)
            subcommand_other "$@"
            exit 0
            ;;
        *)
            echo "unknown subcommand: $1" >&2
            done=1
            exit 1
    esac
done
