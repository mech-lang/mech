# Literal allocation setup ABI oracle

This oracle executes the production Apple AArch64 allocation-setup pass against
the checked runtime. It generates 15 independent native wrappers, including
repeated literal geometry across separate entries. The 99 execution checks cover
all four supported element widths, initialized and uninitialized allocation,
noncanonical true bits, empty capacity, overflow, null contexts, dynamic geometry,
unsupported geometry and a different provider. Allocation counters and physical
cleanup are compared with the original checked entry.

The generated call sites use the production direct-link convention. The
specialization retains the original result values and status checks. Its private
leaf routines supply three literal arguments and tail-branch to the original
allocator without changing the stack or return address.

On an Apple arm64 Mac with the repository dependencies available:

```sh
cargo +nightly-2026-03-03 build --locked --release \
  --config profile.release.package.libmimalloc-sys.opt-level=1 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --target-dir /tmp/mech-allocation-abi-compiler \
  --message-format=json --bin compile_profile \
  > /tmp/mech-allocation-abi-compiler.jsonl
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/apple-m3-win-recovery-20261118/abi/reproduce.py \
  --compiler-build-log /tmp/mech-allocation-abi-compiler.jsonl \
  --output /tmp/mech-allocation-abi-check
```

The output directory contains the actual object, executables, source hashes,
compiler dependency identities, commands and verification logs. This is a
correctness oracle; its command durations are not benchmark results.
