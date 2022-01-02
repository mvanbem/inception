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

function subcommand_audit {
    pushd pc >/dev/null

    # Clear the cumulative error log.
    : > ../build/inception-pack.err

    for map in \
        d1_trainstation_{01,02,03,04,05,06} \
        d1_canals_{01,01a,02,03,05,06,07,08,09,10,11,12,13} \
        d1_eli_{01,02} \
        d1_town_{01,01a,02,03,02a,04,05} \
        d2_coast_{01,03,04,05,07,08,09,10,11,12} \
        d2_prison_{01,02,03,04,05,06,07,08} \
        d3_c17_{01,02,03,04,05,06a,06b,07,08,09,10a,10b,11,12,12b,13} \
        d3_citadel_{01,02,03,04,05} \
        d3_breen_01
    do
        echo "Checking $map"

        exit_code=0
        cargo -q run -p inception-pack $release_flag -- \
            --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
            pack_map \
            --dst ../build \
            $map \
            >../build/inception-pack.log \
            2>>../build/inception-pack.err \
            || exit_code=$?

        if [ $exit_code != 0 ]; then
            echo "Failed! Rerunning for a backtrace"
            RUST_BACKTRACE=1 cargo -q run -p inception-pack -- \
                --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
                pack_map \
                --dst ../build \
                $map

            echo "Rerun unexpectedly succeeded!" >&2
            exit 1
        fi
    done

    sed -e 's/ in materials.*//g' ../build/inception-pack.err | sort | uniq \
        > ../audit.log

    echo "Success! Deduped audit log: audit.log"
    echo
}

release_flag=--release
while true; do
    case $1 in
        --debug)
            release_flag=
            shift
            ;;
        "")
            subcommand_build
            exit 0
            ;;
        build)
            shift;
            subcommand_build "$@"
            exit 0
            ;;
        audit)
            subcommand_audit
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
