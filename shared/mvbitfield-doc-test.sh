#!/bin/bash
cargo watch -d 0 -s "{
    TRYBUILD=overwrite chronic cargo test --color=always \
        -p bitint-macros \
        -p bitint \
        -p mvbitfield-macros \
        -p mvbitfield \
        --all-features
    TRYBUILD=overwrite chronic cargo doc --color=always \
        -p mvbitfield \
        --all-features
} |& less -cR"
