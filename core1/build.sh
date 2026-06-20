#!/bin/bash
if [[ "$1" == "clean" ]]; then
    cargo clean
fi
cargo build --release &&
rust-objcopy.exe -O binary target/thumbv8m.main-none-eabihf/release/core1 target/thumbv8m.main-none-eabihf/release/core1.bin &&
picotool load -v target/thumbv8m.main-none-eabihf/release/core1.bin -o 0x10200000
