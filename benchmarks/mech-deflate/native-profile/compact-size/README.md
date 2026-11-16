# Compact compiler and the 0.970× Roc build

This directory accompanies the compiler-source checkpoint for PR #992. The published Apple M1 build emits 55,956 bytes of machine code, or **0.970× Roc** and **1.361× libdeflate**. The exact object has SHA-256 `a917aa339eb7291e0cbcb5fadedd2a4bf263df0db0ffe408f96310fa084b1f14`. The full timing and correctness evidence is in the benchmark handoff. [Published evidence](https://github.com/mech-lang/mech/tree/39e2a9303e933f7de6e4c4f41ce49b1fcc7a84fe)

## Use this compiler as a base

Start from the PR branch `codex/select-compact-object-bodies`. It carries the checked-region, compact view, and LLVM lowering changes under `hosts/gpu/src/native_artifact`. The branch's original #992 commit is `39cd147bb5`. Its common base with #973's compilation-speed branch at the time of this handoff is `9cb89a2a8e`.

```sh
git fetch origin codex/select-compact-object-bodies
git switch -c compiler-speed-on-compact origin/codex/select-compact-object-bodies
```

For in-progress parser work, rebase a copy of its working branch onto this ref. The benchmark source is `fixtures/source.mec` and the emitter accepts canonical Program bytecode from a parser independently of the parser's implementation.

## Build the exact object

Run the following from the repository root on Apple ARM64 with Python 3.11 or newer and LLVM 22.1.8. Choose an unused output directory. The script checks the build against the published SHA-256.

```sh
python3 -B benchmarks/mech-deflate/native-profile/compact-size/rebuild.py /tmp/mech-roc-size \
  --llvm-bin "$(brew --prefix llvm)/bin"
```

This generates the shared checked returns and the machine-code object from the pinned prepared object and LLVM IR. Build the emitter and add `--emitter` to regenerate those inputs through the compiler source as well.

```sh
cargo +nightly-2026-03-03 build --locked --release \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --target-dir target/compact-size --bin emit_compact_size
python3 -B benchmarks/mech-deflate/native-profile/compact-size/rebuild.py /tmp/mech-roc-size-from-compiler \
  --llvm-bin "$(brew --prefix llvm)/bin" \
  --emitter target/compact-size/release/emit_compact_size
```

The source build reads `fixtures/program.mecb` by default. Pass `--program` with the new parser's output to compare against the same benchmark. The exact recipe verifies the prepared Object, its metadata, and the checked LLVM IR before applying the recorded native layout. For a changed Plan, run `emit_compact_size PROGRAM.mecb OUTPUT_DIRECTORY` to produce fresh compact output and remeasure its layout choices.

The benchmark uses the compact diagnostic entry. `rebuild.py` records the experimental LLVM options and uses the original relocatable-object composition for the exact published object. The whole-object partial-link path in the compiler source retains unwind sections for further integration work.
