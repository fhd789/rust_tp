#!/usr/bin/env bash
set -euo pipefail

cargo bootimage

BOOTIMAGE="target/x86_64-kernel/debug/bootimage-kernel-rust-slab-Fahed-MASAD.bin"

qemu-system-x86_64 \
  -drive format=raw,file="${BOOTIMAGE}" \
  -serial stdio \
  -display none
