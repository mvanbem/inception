#!/bin/bash
set -e

function subcommand_clean {
    rm -rf build/*
}

function subcommand_pack_all_maps {
    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack_all_maps \
        --dst ../build \
        "$@"

    popd >/dev/null

    mkdir -p ftp
    rm -rf ftp/*
    cp ../maps.txt ftp/
    cp -r build/maps ftp/
}

function subcommand_build {
    echo === Building bsp-loader-gx ===
    pushd gc_wii >/dev/null

    cargo build -p bsp-loader-gx $release_flag --no-default-features --features=gamecube,dvd_loader
    elf2dol \
        target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
        ../build/bsp-loader-gx_gamecube.dol

    # cargo build -p bsp-loader-gx $release_flag --no-default-features --features=wii,dvd_loader
    # elf2dol \
    #     target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
    #     ../build/bsp-loader-gx_wii.dol

    popd >/dev/null

    cp build/bsp-loader-gx_gamecube.dol ftp/bsp-loader-gx.dol


    echo === Building apploader ===
    pushd gc_wii >/dev/null

    cargo build -p apploader --release

    cp target/powerpc-none-eabi/release/apploader ../build/

    popd >/dev/null


    echo === Building disc image ===
    pushd build >/dev/null

    mkdir -p disc_root
    rm -rf disc_root/*
    cp ../maps.txt disc_root/
    mkdir disc_root/maps
    cp maps/* disc_root/maps/

    popd >/dev/null
    pushd pc >/dev/null

    cargo run -p build-gcm -- \
        --apploader ../build/apploader \
        --dol ../build/bsp-loader-gx_gamecube.dol \
        --root-directory ../build/disc_root \
        --output ../build/inception.gcm

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

    cat ../maps.txt | while read map; do
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
release_path_component=release
while true; do
    case $1 in
        --debug)
            release_flag=
            release_path_component=debug
            shift
            ;;
        ""|build)
            subcommand_build
            exit 0
            ;;
        audit)
            subcommand_audit
            exit 0
            ;;
        clean)
            subcommand_clean
            exit 0
            ;;
        pack_all_maps)
            subcommand_pack_all_maps
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
