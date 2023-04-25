#!/bin/bash
cargo watch -d 0 -s "{
    TRYBUILD=overwrite chronic cargo test --color=always \
        --all-features \
        -p bitint-macros \
        -p bitint \
        -p bitint-test-checked \
        -p mvbitfield-macros \
        -p mvbitfield
    chronic cargo test --color=always -p bitint-test-unchecked --profile=test-unchecked
    chronic cargo doc --color=always \
        --all-features \
        -p bitint \
        -p mvbitfield
} |& less -cR"
