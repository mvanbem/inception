#!/bin/bash
set -e

svd2rust -i gamecube.svd --target=none
rm -rf src
form -i lib.rs -o src/ && rm lib.rs
cargo fmt
