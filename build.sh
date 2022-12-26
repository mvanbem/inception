#!/bin/bash
set -e

function subcommand_clean {
    rm -rf build/*
}

function subcommand_pack_map {
    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack-map \
        --dst ../build \
        "$@"

    popd >/dev/null

    mkdir -p ftp
    cp -r --preserve=timestamps assets/maps.txt build/maps ftp/
}

function subcommand_pack_all_maps {
    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack-all-maps \
        --dst ../build \
        "$@"

    popd >/dev/null

    mkdir -p ftp
    cp -r --preserve=timestamps assets/maps.txt build/maps ftp/
}

function subcommand_pack_model {
    echo === Running inception-pack ===
    pushd pc >/dev/null

    cargo run -p inception-pack $release_flag -- \
        --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
        pack-model \
        --dst ../build \
        "$@"

    popd >/dev/null

    mkdir -p ftp
    cp -r --preserve=timestamps build/models ftp/
}

function subcommand_build_embedded {
    echo === Building bsp-loader-gx ===
    pushd gc_wii >/dev/null

    cargo build -p bsp-loader-gx $release_flag --no-default-features --features=gamecube,embedded_loader
    elf2dol \
        target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
        ../build/bsp-loader-gx_gamecube.dol

    # cargo build -p bsp-loader-gx $release_flag --no-default-features --features=wii,embedded_loader
    # elf2dol \
    #     target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
    #     ../build/bsp-loader-gx_wii.dol

    popd >/dev/null

    mkdir -p ftp
    cp --preserve=timestamps build/bsp-loader-gx_gamecube.dol ftp/bsp-loader-gx.dol

    echo === SUCCESS ===
}

function subcommand_build_ftp {
    echo === Building bsp-loader-gx ===
    pushd gc_wii >/dev/null

    cargo build -p bsp-loader-gx $release_flag --no-default-features --features=gamecube,ftp_loader
    elf2dol \
        target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
        ../build/bsp-loader-gx_gamecube.dol

    # cargo build -p bsp-loader-gx $release_flag --no-default-features --features=wii,ftp_loader
    # elf2dol \
    #     target/powerpc-none-eabi/$release_path_component/bsp-loader-gx \
    #     ../build/bsp-loader-gx_wii.dol

    popd >/dev/null

    mkdir -p ftp
    cp --preserve=timestamps build/bsp-loader-gx_gamecube.dol ftp/bsp-loader-gx.dol

    echo === SUCCESS ===
}

function subcommand_build_gcm {
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

    cp --preserve=timestamps build/bsp-loader-gx_gamecube.dol ftp/bsp-loader-gx.dol


    echo === Building apploader ===
    pushd gc_wii >/dev/null

    cargo build -p apploader --release

    powerpc-eabi-objcopy -O binary target/powerpc-none-eabi/release/apploader ../build/apploader

    popd >/dev/null


    echo === Building disc image ===
    pushd build >/dev/null

    mkdir -p disc_root
    rm -rf disc_root/*
    cp -r --preserve=timestamps ../assets/{opening.bnr,maps.txt} maps disc_root/

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

function subcommand_build_kernel_gcm {
    echo === Assembling kernel ===
    pushd gc_wii/kernel >/dev/null

    ./assemble.sh

    popd >/dev/null


    echo === Building kernel ===
    pushd gc_wii >/dev/null

    cargo build -p kernel $release_flag

    elf2dol -v -v \
        target/powerpc-none-eabi/$release_path_component/kernel \
        ../build/kernel.dol
    cp --preserve=timestamps ../build/kernel.dol ../ftp/kernel.dol

    popd >/dev/null


    echo === Building apploader ===
    pushd gc_wii >/dev/null

    cargo build -p apploader --release

    powerpc-eabi-objcopy -O binary target/powerpc-none-eabi/release/apploader ../build/apploader

    popd >/dev/null


    echo === Building disc image ===
    pushd build >/dev/null

    mkdir -p kernel_disc_root
    rm -rf kernel_disc_root/*
    cp -r --preserve=timestamps ../assets/opening.bnr kernel_disc_root/

    popd >/dev/null
    pushd pc >/dev/null

    cargo run -p build-gcm -- \
        --apploader ../build/apploader \
        --dol ../build/kernel.dol \
        --root-directory ../build/kernel_disc_root \
        --output ../build/kernel.gcm

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

    cat ../assets/maps.txt | while read map; do
        echo "Checking $map"

        exit_code=0
        cargo -q run -p inception-pack $release_flag -- \
            --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
            pack-map \
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

    sed -e 's/ in materials.*//g' -e 's/clamped.*/clamped/g' ../build/inception-pack.err | sort | uniq \
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
        ""|build|build-embedded)
            subcommand_build_embedded
            exit 0
            ;;
        build-ftp)
            subcommand_build_ftp
            exit 0
            ;;
        build-gcm)
            subcommand_build_gcm
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
        pack-map)
            shift
            subcommand_pack_map "$@"
            exit 0
            ;;
        pack-all-maps)
            subcommand_pack_all_maps
            exit 0
            ;;
        pack-model)
            shift
            subcommand_pack_model "$@"
            exit 0
            ;;
        build-kernel-gcm)
            subcommand_build_kernel_gcm
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
