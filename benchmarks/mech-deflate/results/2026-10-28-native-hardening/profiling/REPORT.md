# Native compression profile and binary inspection

These are diagnostic profiles on Apple M1, ARM64, macOS 15.6.1. They identify
costs to investigate; they are not a controlled language-speed comparison or
qualification of the new PR head. Both compressors use level 12 and the same
eight literal bytes, `abcdefgh`, with exact raw output verification. No large
compression job was restarted. Allocation bytes below are cumulative requested
bytes, not peak memory or a measurement of every copied byte.

## Revisions and boundaries

The retained complete Mech executable is from
`c1d20645d7b58f6b21847f51bfbf37225b6e6679`, binary SHA-256
`6c23c247bb0c661e3627a7ca833ca6f724139b1b4e3d13d1008239f0b1393346`.
Its build/source pairing is preserved in
[the earlier executable receipt](../../2026-10-27-complete-output-reuse/level12-maintained-repeated-output-executable.json).
The saved revision-14 artifact SHA-256 is
`a2daaaf746d6bcc8668c5439f84a782ecea2aff376546566c01d4fc0f8c6f585`.
The focused word probe and Object inspection link the cached optimized compiler
libraries from `fce2c2c9c3ff9752460375e31c2e6c68225f84b0`. GPU/runtime code is
identical between those revisions; the intervening engine change concerns source
tuple annotations. These observations are explicitly separate from fresh tests
at the hardening commits. Warnings remained errors when building the probe.

Roc package revision is `671077457ff7ef3a3d44caf715a3fc595addf379`, compiler
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, built with `--no-cache --opt=speed`.
Its measured executable SHA-256 is
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
The maintained harness clears/reuses output, warms for 200 ms, and verifies
20,000 repeated compressions. Only its repetition count was changed for
sampling. Roc links its pinned mimalloc; Mech uses System with an allocation
probe. Repetition counts, allocator choice and sampling perturbation differ, so
elapsed totals are not a speed ratio.

Mech JIT compilation took 38.581 seconds and is outside its compression timer.
The sampled compression completed in 2.802 seconds, requested 16,658,509
allocations totalling 1,760,520,789 bytes, produced the expected ten raw bytes,
passed independent inflation and input-alias checks, and released all managed
memory. Sampling began at the compression marker and captured 1,894 active
compression-thread samples. The historical long-input sample is a diagnostic
only: that full-corpus run was cancelled without a result.

## Measured generic word updates

The probe compiles an ordinary function with one or eight distinct indexed word
assignments. It retains an input alias, verifies every output/input word and
complete managed cleanup. Source, input construction, compilation, verification
and output destruction are outside evaluation timing. Each row is the median
of three diagnostic observations, with no performance-acceptance claim.

| Buffer | Stores | Seconds | Requests | Requested bytes |
|---|---:|---:|---:|---:|
| U16 × 288 | 1 | 0.000107459 | 1,366 | 156,520 |
| U16 × 262144 | 1 | 0.013473500 | 1,366 | 680,232 |
| U16 × 288 | 8 | 0.000561042 | 9,913 | 1,112,688 |
| U16 × 262144 | 8 | 0.105309292 | 9,913 | 5,302,384 |
| U32 × 288 | 1 | 0.000087083 | 1,366 | 157,096 |
| U32 × 1500513 | 1 | 0.075290250 | 1,366 | 6,157,996 |
| U32 × 288 | 8 | 0.000535916 | 9,913 | 1,117,296 |
| U32 × 1500513 | 8 | 0.590805334 | 9,913 | 49,124,496 |

At each width, extra requested bytes equal exactly the number of stores times
the extra payload size. The maintained packed indexed-assignment constructor
calls `words.to_vec()` for every update. This establishes the full-buffer copy
cost in allocation traffic. It does not attribute all elapsed time to copying:
the projected-output path also visits elements and repeatedly charges/checks
the budget. Preserving an external alias requires the first private copy, but
does not inherently require another full copy for every later private write.

## Profiles and generated code

Mech's sampled stacks repeatedly reach `indexed_assign_snapshot`,
`projected_snapshot_matrix_output`, `charge_compute_work`,
`check_planning_limits`, `check_turn_planning_progress`, and
`evaluate_memory_budget`, with TLS/lock and allocation activity. Prepared call
and binding caches already exist and are active; adding an undifferentiated
cache is not the finding. Inclusive stack categories overlap and must not be
summed as independent percentages.

The emitted Mech computation contains 2,999,140 code bytes, 749,785 ARM
instructions and 52,189 static indirect call sites. These are static counts,
not dynamic call totals. Its graph has 100,256 generated blocks and 525,920
instructions; the recorded CFG cleanup removes no constant branches. The
compiler-host executable is 66 MB, but includes the parser, compiler and harness
and is not the guest-code size.

The complete Roc executable's `__text` is 195,456 bytes, including its allocator,
runtime and harness. Its hottest function is heavily inlined. The sampled hot
offsets include typed initialization loops writing 40 bytes per iteration, with
bounds matching the real 524288-byte tree and 6002052-byte cache. This is useful
code-generation evidence, with different packaging from Mech's emitted entry.

The reference's general list implementation distinguishes exclusive ownership
and uses an in-place typed element replacement when allowed; shared inputs take
the ordinary uniqueness/copy path. The transferable ideas are general typed
ownership and direct scalar/buffer operations. Any Mech refinement must retain
current shapes, exact-width checks, aliases, error order, finite admission,
failure cleanup and retry. Removing runtime checks or recognizing compressor
names/sizes would not implement that contract.

The separately declared mixed-frame study isolates frame overhead before
changing it. Indexed-write copying and repeated footprint/budget traversal are
demonstrated later targets; this report does not claim they are fixed or that
frame overhead accounts for the complete compressor's cost.

## Evidence

`MANIFEST.json` records hashes of the retained inputs, sources, receipts,
disassembly excerpts and compressed raw samples. `word-updates.jsonl` retains
all observations. The Object helper emits code only. No object or executable is
checked into the repository. The first probe build's unused-import/dead-code
warnings were fixed before its successful `-D warnings` build. The CPU-model
read initially required broader read permission; the retried read returned
Apple M1. No profiling access or runtime resource denial occurred in these
completed probes.
