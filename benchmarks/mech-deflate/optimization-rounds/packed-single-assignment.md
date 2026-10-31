# Round 02: single packed assignment planning

Declared before implementation and new timing. Starting compiler/reviewed head:
`376cabbe2d204e9f12ad3b52c9f26b56dd6988ec`. Its Rust implementation matches
`c24984b402a1228cccf934a478965143b539f611`. Review completed without findings:
https://github.com/mech-lang/mech/pull/973#issuecomment-6467379536.

The complete eight-byte profile reaches indexed assignment's projected-output
walk and repeated budget checks. A single tree/cache write traverses every
destination, despite the fixed-width result retaining the same shape and size.
The existing real-buffer witness uses 262144 U16 and 1500513 U32 words, one/eight
stores, immutable input aliases, exact outputs and complete cleanup. This is a
general packed assignment optimization, not a new compressor operation or IR.

Minimal source operation:

```mech
touch(buffer<[u32]>) => (out<[u32]>) :=
  ~next := buffer
  next[1u64] = 7u32
  out := next.
touch(input<[u32]>)
```

The compiler operation unblocks efficient real tree/cache updates. The pinned
compressor's source, algorithm, tables, search/EOF limits, buffers, allocations
and timing boundaries remain unchanged. The matched allocator stays pinned.

Implement only the proven one-selected-destination/one-source-element case for
actual packed unsigned matrices. Keep ordinary operand/selector validation and
current-value finalization preflight in their existing order. Derive the exact
single source's bounded primitive finalization cost; preview the complete
planning charge through the existing meter owner. Prepay only when it fits.
Otherwise retain the original incremental traversal and its first-denied
requirement, already-spent prefix and error precedence. No output construction,
copy admission, ownership, equality or publication check is removed.

Acceptance covers U8/U16/U32/U64/U128, scalar and one-cell packed sources,
row/column geometry, nonfirst positions, mismatched/unsupported layouts,
ordinary/fallback plan agreement, finite boundary denials and spent work,
aliases, unchanged publication and retry. Multiple selections retain the
existing path. Use original/decoded resident/JIT regressions, plus the real
buffer witness. Protect ordinary scalar execution and runtime-only boundaries.

After correctness, compare the preserved round-01 executable with the new one
using the same eight-byte artifact and unchanged timing harness. Each process
has one warmup, three retained compression calls and one excluded profile call.
Run alone, with no local compiler or old compressor active. Keep every sample,
setup failure and raw profile. Refresh the pinned reference's 20000 samples.
If a single excluded call is too short for sampling, use a separate post-primary
profile process with enough repetitions for about four seconds of compression,
chosen from the primary median. Its observations never enter the primary median.
Report all elapsed times divided by that reference (Roc = 1x); also retain the
round-01 5756.19x anchor. This is a small diagnostic, not a statistical campaign.
The complete program's reported logical work must remain 939091080723 and all
bytes, aliases, capacity and cleanup checks must pass. A result that changes
those contracts fails acceptance even if it runs faster.

Commit implementation, raw measurements, executable/source/allocator hashes and
the post-change profile before another optimization round. Do not run a large
input or full corpus during this round. The goal remains verified 1x parity.

The optimized engine unit build completes in 1082.404 seconds; its three tests
execute in 0.01 seconds. All finite-prefix and live-ledger comparisons pass.
The combined profile also builds the GPU unit target but selects zero tests
there; that zero selection is not counted as qualification. Native component,
linked and complete eight-byte checks follow before broader qualification.
