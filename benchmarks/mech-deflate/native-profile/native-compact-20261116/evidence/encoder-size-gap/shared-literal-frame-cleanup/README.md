# Recovered final cleanup and shared allocation arguments

The Apple M1 encoder emits **64,444 B**, down **208 B** from the qualified
64,652 B heap-layout recipe. That is **1.117× Roc** and **1.738× libdeflate**.
The checked entry shrinks by 340 B and helpers/alignment grow by 132 B.
Peak native stack falls from 4,784 B to **4,656 B**.
[Size attribution](size-attribution.json) · [Source and profiles](candidate.json)

Both complete orders record **81/81 Roc and 62/81 libdeflate point wins**,
covering **74/74 and 56/74 distinct contracts**. The unchanged v3 gate
returns **Retain**, with no lost peer cases. Precision supports 81 Roc wins
in each order and 62 C wins forward, **61 C wins reverse**. This result does
not establish the same precision margin on every C win.
[Decision and tradeoffs](result.json) · [Full replay](full-replay.json.gz) ·
[Every timing in seconds and comparator multiples](timings.csv)

This remains an opt-in source recipe. The maintained frame-copies control
is 71,192 B. The original Mech program, canonical bytecode, timed native
boundary, fixed runtime, allocator and peer executables are unchanged.
Runtime timing includes heap construction, allocation transfer and physical
private cleanup. Complete raw observations are in the forward and reverse
tar archives beside this document.

## What changed

The earlier final-CFG cleanup pass still saves 96 B on the 64,652 B recipe.
The independent literal-allocation setup option adds 48 B when combined
with that recipe's existing heap-frame stubs. Combining the heap argument
and literal geometry restoration in the same private stub instead saves
48 B. Its combination with final cleanup saves 208 B overall.

Only direct five-word allocation imports with the exact two-word result
ABI, the same entry heap value, proved I64 stride/alignment constants in
1, 2, 4 or 8, and a proved zeroing constant qualify. Capacity stays dynamic.
The private leaf restores the original arguments and tail-branches to the
original checked allocator, preserving SP, LR, status and owner results.
The original nonzero interpretation of zeroing remains intact.

The compiled ABI check covers all **26 allocation calls** and all **259
shared heap calls**. Final cleanup removes one unreachable nullable-release
call. Linked checks retain 162 successful exact-output/decompression/alias
observations and the six injected allocation-failure cases with matching
origins and private cleanup. The source rebuild additionally checks exact
recovery after each injected failure.
[Compiled ABI proof](abi-proof.json) · [Linked contracts](linked-checks.json) ·
[Implementation plan](implementation-plan.md)

## Runtime tradeoffs

Compared with 64,652 B, the replay reports two established control
slowdowns and ten uncertain cases. The combined slowdown estimates are
**2.219% for empty** and **0.964% for 320-overlap**. Their per-case 90%
intervals are 1.681–2.775% and 0.574–1.506%. These are not simultaneous
confidence bounds. All diagnostics and both-order observations remain in
the replay. Peer qualification does not imply per-case nonregression.

Natural 8 KiB remains below C at 0.982943× forward and 0.986542× reverse.
The previous Roc loss at natural 32,769 B remains a win at 0.961837× and
0.965623× Roc. Overlap's point ratios are 0.995445× and 0.992121× C.

## Recovering cleanup work

The named source recipe selects `MECH_COMPACT_FINISH_CFG_CLEANUP=batched-routes`.
It reconstructs current CFG and dominance facts for each loop/identity
rewrite, retires unreachable routes and constant phis between them, then
runs one ordered finishing sweep and the adjacent-address pass. This
avoids repeating the broad ordered schedule after each route rewrite.
Current graph proofs and unconditional SSA verification remain in place.

The selected compiler source is
`caa8b22ac5cd4d7c3cf30b5c97f78fd786fe5c16`. It reproduces all three
qualified program artifacts and the exact **f77ad506…** linked worker.
The original runtime qualification was acquired with the compiler at
`4f5a94ac3f0546c99f5f88eb9b500f7bd0266527`. These are distinct compiler
identities producing the same complete worker.
[All schedule screens](cleanup-recovery/batched-screen.json) ·
[Exact worker links](cleanup-recovery/exact-workers.json) ·
[Recovery plan](cleanup-recovery/plan.md)

The first ordered-only schedule emits 64,556 B and changes artifacts.
Adding the three imported searches that actually changed the graph
reproduces 64,444 B, but saves only a paired 8.012 ms and still adds
97.167 ms versus the 64,652 B control. The batched schedule retains the
exact worker while reducing repeated ordered cleanup. A leaner variant
emits the same byte count with different code and remains unqualified.
[Ordered-only screen](cleanup-recovery/ordered-only-screen.json) ·
[First recovery's matched builds](cleanup-recovery/routes-only/deployment.json)

Ten balanced builds with the selected compiler measure **2.263127 s**
through final link and **2.273977 s** through strip. Batching saves a paired
median **17.200 ms** against the full schedule, with 8/10 faster. It still
adds **94.835 ms**, or **1.0437×**, versus the 64,652 B control. Code size
improves while the extra build cost remains a target for further work.

| Same-session actor | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: |
| 64,652 B control | 2.169865 s | 2.180720 s | 2.198015 s | 392.5 MB |
| 64,444 B full cleanup | 2.280134 s | 2.290961 s | 2.309317 s | 392.8 MB |
| 64,444 B batched cleanup | 2.263127 s | 2.273977 s | 2.291872 s | 392.8 MB |
| Roc | 2.977284 s | 2.988122 s | 2.935612 s | 392.2 MB |
| libdeflate | 0.480397 s | 0.491348 s | 0.442004 s | 71.5 MB |

The selected recipe takes **0.7603× Roc** and **4.7158× libdeflate**
build time. Phases are 0.706344 s parsing, 0.212691 s source semantics,
0.009339 s artifact/encoding, 0.000017 s catalog join wait and 1.272847 s
native emission. Link takes 0.035060 s. Catalog construction still runs
during parsing; its join wait is not the CPU work removed.
[Selected build measurements](cleanup-recovery/deployment.json) ·
[All source/link trials](cleanup-recovery/build-comparison.json.gz)

## Original full-cleanup build cost

Ten balanced fresh-process builds per actor use one compiler built from
`4f5a94ac3f0546c99f5f88eb9b500f7bd0266527`, the original sources and the same
prebuilt deployment support. Every trial verifies object, bytecode,
complete metadata and stripped deployment identity. The full final-cleanup
schedule costs a paired median **111.002 ms**, or **1.0504×** the 64,652 B
control. Its smaller code does not make this build faster.

| Apple M1 actor | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Heap-layout control | 64,652 B | 2.202147 s | 2.213113 s | 2.228227 s | 392.9 MB |
| Cleanup alone | 64,556 B | 2.312167 s | 2.323456 s | 2.332766 s | 393.3 MB |
| Literal frame and full cleanup | 64,444 B | 2.310807 s | 2.322202 s | 2.332187 s | 393.3 MB |
| Roc | 57,696 B | 3.003969 s | 3.015121 s | 2.960111 s | 393.2 MB |
| libdeflate | 37,080 B | 0.489356 s | 0.500253 s | 0.445723 s | 76.2 MB |

The full-cleanup candidate takes **0.7685× Roc** and **4.7286× libdeflate**
source-to-link time. Its compiler phases are 0.713074 s parsing, 0.218849 s
source semantics, 0.009317 s artifact/encoding, 0.000021 s catalog join wait
and 1.299900 s native emission. Native emission in the control is 1.186526 s.
Phase medians need not sum to the complete wall-time median. Compiler and
runtime bootstrap builds are outside this matched timing boundary.

All three Mech deployment files remain **276,072 B** after the same strip.
The 208 B encoder reduction does not cross a linked-file size boundary.
[Matched measurements](deployment.json) · [Raw builds](build-comparison.json.gz)

## Earlier passes revisited

The audit repeats earlier mechanisms on the 64,652 B base. All-output
pruning is re-evaluated after remapping all 24 controller windows and all
32 continuation windows, rather than treating stale selections as a size
rejection. That valid result is 32 B larger. Six-word continuations are
16 B larger. Broader backing-owner proofs save 32 B, final cleanup saves
96 B, and their combination saves only 16 B. These are static screens,
except cleanup alone also has both focused runtime orders.
[Full audit and receipts](revisited-gains-audit.json)

## Source rebuild

From this benchmark packet directory in a complete PR #995 checkout on
Apple AArch64, use the fixed comparator allocator object.

```sh
python3 -B rebuild.py /tmp/mech-m1-literal-frame-cleanup \
  --variant m1-literal-frame-cleanup --mimalloc-object /path/to/mimalloc.o
```

This builds the compiler from source, verifies all three program artifacts,
links the worker and checks success, failures, aliases, cleanup and recovery.
Its receipt records the newly linked worker independently of the archived
benchmark worker. Fixed-support links are used for the runtime qualification
and the deployment timing above. No LLVM dependency is added.

The command passes **162 success observations, six injected failure cases
and 12 exact recovery calls**. The new source-built worker is `80feaed1…`;
its runtime wins are not measured by this command. The complete runtime
counts above belong to the exact fixed-support `f77ad506…` worker. The
separate fixed-support link reproduces that worker with the selected compiler.
[Source rebuild receipt](source-rebuild.json) · [Successes](source-successes.json) ·
[Failures](source-failures.json) · [Recovery](source-recoveries.json)

Workspace formatting, warning policy and Rust module-layout checks pass.
The local unsafe-boundary check stops at its missing allowlisted historical
file `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`.
Required CI remains separate from these local checks.
