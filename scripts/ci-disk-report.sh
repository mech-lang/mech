#!/usr/bin/env bash
set -euo pipefail

df -h
du -sh target 2>/dev/null || true
du -sh "${CARGO_HOME:-$HOME/.cargo}" 2>/dev/null || true
du -sh "${RUSTUP_HOME:-$HOME/.rustup}" 2>/dev/null || true

if [[ -d target ]]; then
  du -xh --max-depth=2 target 2>/dev/null |
    sort -h |
    tail -30
fi
