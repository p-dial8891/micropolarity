#!/bin/bash
if [[ "$2" == "clean" ]]; then
    cargo clean
fi
cargo build --features=$1 --release &&
rust-objcopy -O binary target/thumbv8m.main-none-eabihf/release/core1 target/thumbv8m.main-none-eabihf/release/core1.bin &&
sudo /mnt/BeeFs/Projects/micropolarity/picotool/picotool load -v target/thumbv8m.main-none-eabihf/release/core1.bin -o 0x10200000
