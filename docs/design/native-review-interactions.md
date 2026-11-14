# R261: native interaction repairs

**Retain the four correctness repairs.** The repaired experimental compiler emits
the same compression Object and linked worker as R256. No new optimization is
enabled, and the new timing observations establish no speedup or slowdown caused
by these fixes.

Reviewed published head: `9dbdf42d2157715941e3b6cd75421eb01442d1d7`.
PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
Archived compiler base: `608ed69aedb98bb8c9d61b0bf6364580b304a6d1` (R256).
IR/work/ABI/bytecode remain 215/107/39/14. The R255 backend and pinned dependencies
are unchanged. The ordinary tracked runtime and archived compiler have separate
source manifests and validation profiles.

## Repairs and review scope

| Finding | Change | Revision or packet |
|---|---|---|
| OPT-01, P1 | Exclude pack-internal scalar snapshots from both repeated-read and operand reuse. Packing can bypass those definitions; cache misses keep the original reads and checks. | R261 archived compiler patch |
| OPT-02, P2 | Use current logical matrix cardinality in both managed field admission and parent numeric alias preflight. An insufficient parent preflight declines to ordinary admission at the original call. | `0fae50df8ebc518356ccf753e568271cc2c0604a` |
| OPT-03, P2 | Acquire each input owner only when that source home is live at entry. Clear unused owner homes while preserving length, pointer, capacity and geometry metadata. Genuinely live aliases retain separate tokens. | R261 archived compiler patch |
| OPT-04, P2 | Support current review packets and legacy event archives through explicit schema dispatch, full manifest verification and pinned report replay. | `b29753d0842623e85cc480de482c05421ca7d7c1` |

The archived source closure contains 825 files, with two semantic repairs and one
diagnostic metadata change. Its reversible three-file patch reconstructs the
exact R256 closure. Those experimental files are not represented as changes to
ordinary tracked files that lack this machinery. R260's rejected performance
controls remain separately archived in `d881839cb5a699e96e34d9a2c2d04e3cd78c245b`.

The projection tests compare with actual ordinary selected execution. Empty
matrices retain its minimum one publication element. The parent fixture's earlier
two-field call retains its required-two error at allowance one; allowance two
reaches the required-three projection error. Test-only controls decline nested
phase selection so the parent shortcut itself executes. No public setting,
serialized authority or production selection behavior changes.

## Native qualification

| Validation | Observed result |
|---|---:|
| Archived compact Rust matrix | 715 passed, 7 inherited exclusions |
| Current maintained Rust matrix | 680 passed, 7 inherited exclusions |
| Focused projection tests | 2 passed; both actual shortcuts selected |
| Existing record test plus live allocation-probe control | 2 passed |
| Pack/reuse original and decoded native calls | 75,600 passed, plus the exact review example |
| Real-owner original and decoded controls | 12 passed |
| Maintained replay unit tests | 6 methods passed, including current and legacy formats |
| Actual R256–R260 maintained CLI replays | 11,647 file occurrences; 33 identical reports |
| Complete compression original/decoded smokes | 324 calls; all 81 cases through 1 MiB |

Both Rust test roots use `-C opt-level=1 -D warnings`; the performance GPU library
uses `-C opt-level=3`. Their features are `aot`, `jit`,
`canonical-native`, `canonical-native-runtime`, `canonical-native-program` and
`canonical-native-program-runtime`. Exact build and test commands are in
`qualification-results.json` and the build receipts in the packet. Layout,
host-catalog, source-catalog, changed Rust formatting and diff checks pass.

Native pre-fix witnesses reproduce the verifier failure, both admission bypasses
and the extra owned-input token that prevents final transfer. Repaired controls
cover widths 2/4/8, row/column/rectangle views, positive insufficient limits,
overflow, assertions, ordinary error precedence, aliases, unchanged publication,
cleanup and retry. Non-pack controls retain safe read and operand reuse. Complete
compression retains exact reference bytes and independent decompression.

The existing zero-copy record test's successful allowance changes from one to
three for its three-element matrix. New controls explicitly require rejection at
one and two. Its original zero-allocation assertion still passes.

## Matched compression measurements

Two fresh balanced cohorts cover every case with 48 blocks per case, ratios
formed within each block, fixed level 12, runtime inputs and equal 200-ms warmups.
Complete compression, scratch construction, output transfer and private cleanup
remain inside the timer. Validation, prepared inputs and external output-slot
lifetime remain outside on both sides. Builds, tests and counters do not overlap
primary elapsed; hardware and heap counters use separate cohorts.

| Input | Bytes | Mech / Roc | Mech median ms |
|---|---:|---:|---:|
| Eight literals | 8 | 0.417756x | 0.118083 |
| Natural text | 32768 | 1.837612x | 7.981396 |
| Random | 32768 | 1.816686x | 1.924104 |
| Periodic | 32768 | 1.081951x | 1.351562 |
| Repeated | 32768 | 1.051936x | 1.405354 |
| Natural text | 1048576 | 1.901775x | 306.181855 |
| Random | 1048576 | 2.169105x | 62.393667 |
| Periodic | 1048576 | 1.265729x | 39.660021 |
| Repeated | 1048576 | 1.221235x | 41.845354 |

All 81 existing 1.05 gates pass for both pooled medians and block medians.
The paired all-case repaired/before geometric ratio is 0.99417181x; the
largest adverse paired row is periodic 4096, +1.1520%. These are variation between
identical executables, not measured effects of a new optimization. Heap request,
requested-byte, initialized-copy, zeroing and private-peak fields match in all 81
cases. Final nonreturned private bytes remain zero.

Object SHA-256: `1f4b7d43df21629b96e3c955fe6a52f9a2b2f75ea81d506450b490c81699e631`.
Linked worker SHA-256: `96091f758139371d0f8455d18be30c2086af70904c5604a0b11307c651648297`.
Both match R256 exactly. Full hardware/memory counters and all 81 rows remain in
the packet; requested capacity is not copied or live byte volume.

## Limits and evidence

The compact timing entry still omits the ordinary managed resource/accounting/
publication/recovery contract. The maintained correctness tests are separate;
these compression timings do not quantify the restored projection checks' cost
on that managed route. Source algorithms, allocation sites, tables, search limits,
real scratch capacities, allocator and timing boundaries are unchanged.

Seven pre-existing Rust exclusions remain explicit in `inherited-exclusions.json`;
none is newly waived. The monolithic integration build's reconstructed profile
lacked `mech_build`. The exact selected record test and its shared helpers passed
separately; full integration, linked/AOT regeneration and full CI are not claimed.
Two disk-exhausted Engine builds passed with the same configuration after old
build cleanup. Earlier fixture and harness errors are retained in the failure
ledger. No introduced semantic failure remains unresolved. No new function-level
profile, LLVM comparison or full 6.6-MB run is claimed.

[Public R261 packet](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r261-interaction-repairs-a46590f0db68.tar.gz)
contains 2,042 hash-verified files, the complete source closure, patches, fixtures,
commands, negative and passing receipts, aggregate observations and five replayed
reports. Fresh public retrieval verifies all files and exact bidirectional source
reconstruction. SHA-256:
`a46590f0db68b06cb2bbeeef81cedee332f1f771ade27d774bd104c5812bb991`.
Native binaries, bytecode, full compressor descriptors/IR/maps, raw traces and
credentials are excluded.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round \
  --index benchmarks/mech-deflate/experiments.json --round 261 \
  --output /tmp/mech-r261-review
```

This command verifies and replays evidence; it does not execute a compiler or
compressor. The PR stays draft pending review. Obtain review of these interaction
repairs before the next semantic compiler change.
