#!/bin/bash
set -e

rm -rf build/*

echo === Running inception-pack ===
pushd pc

cargo run -p inception-pack --release -- \
    --hl2-base ~/.steam/steam/steamapps/common/Half-Life\ 2/hl2 \
    --dst ../build \
    "$@"

popd


echo === Converting PNG textures ===
pushd build

gxtexconv -i lightmap.png colfmt=14
mv lightmap{,_cmpr}.tpl
rm lightmap.h

gxtexconv -i lightmap.png colfmt=6
mv lightmap{,_rgba}.tpl
rm lightmap.h

popd


echo === bsp-loader-gx ===
pushd gc_wii

cargo build -p bsp-loader-gx --release --no-default-features --features=gamecube
elf2dol \
    target/powerpc-none-eabi/release/bsp-loader-gx \
    ../build/bsp-loader-gx_gamecube.dol

cargo build -p bsp-loader-gx --release --no-default-features --features=wii
elf2dol \
    target/powerpc-none-eabi/release/bsp-loader-gx \
    ../build/bsp-loader-gx_wii.dol

popd


echo === SUCCESS ===
