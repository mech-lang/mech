#!/usr/bin/env sh
set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

exec cargo run --quiet \
  -p mech \
  --no-default-features \
  --example r-stack-trust-proof \
  --features r_stack_trust_proof \
  -- "$@"
