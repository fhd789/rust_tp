#!/usr/bin/env bash
set -euo pipefail

cargo test --target x86_64-kernel.json
