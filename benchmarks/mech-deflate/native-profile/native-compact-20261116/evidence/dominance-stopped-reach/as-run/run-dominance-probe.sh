#!/bin/sh
set -eu

scratch=/private/tmp/mech-native-work-census.zZUrL7
deps=/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/host-opt-levels/opt3-cargo/release/deps

rustc +nightly-2026-03-03 --edition=2024 --crate-name route_dominance_probe --test \
  -L "dependency=$deps" \
  --extern "cranelift_codegen=$deps/libcranelift_codegen-e20ffdf0b6997069.rlib" \
  "$scratch/dominance-probe.rs" -o "$scratch/dominance-probe"

"$scratch/dominance-probe" --nocapture --test-threads=1
