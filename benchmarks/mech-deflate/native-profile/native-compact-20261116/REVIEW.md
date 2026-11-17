# Review and continuation handoff

Review **mech-lang/mech PR #993**, branch
`codex/native-compact-source-handoff`. It is stacked on #973's
`codex/level12-compression-consolidated` branch. The compiler changes, Mech source,
build recipe, runtime source, and evidence are committed here. Another agent can
fetch this branch and use it as the basis for compiler work.

```sh
git fetch origin codex/native-compact-source-handoff
git switch -c codex/review-native-compact origin/codex/native-compact-source-handoff
```

The retained runtime floors are **81/81 Roc and 61/81 libdeflate**, with 62 an
improvement target. Continue reducing emitted code and complete build time toward
1× libdeflate. Preserve checked errors, origins, ownership, and finite admission;
keep the ordinary resource-accounting limitations explicit. The rebuild recipe
uses no external LLVM executable. Do not substitute old M3/LLVM measurements for
the current Apple M1 comparisons.

## What is retained

The baseline recipe remains unchanged: **72,520 bytes of encoder text**, or
**1.25693× Roc and 1.95577× libdeflate**. Historical full runtime runs in both
orders record 81/81 and 61/81. Rebuilt objects and metadata match those artifacts
exactly; freshly linked worker executables passed correctness checks but have
not received a new full peer timing qualification.

The frontend allocator change is published in `21757538`. Its matched comparison
improved complete build time by 8.83%, with all ten Mech outputs byte-identical.
The parser now reads 32-entry journal pages, hashes token bytes in contiguous
spans with unchanged work limits, and skips an unused map when caching subtrees.
The parser benchmark improves by 16.82% with identical trees and parser steps.
The matched complete build improves from **6.30298 to 5.65845 seconds**
(10.23% less time), reaching **1.89458× Roc and 11.61221× libdeflate**.
The retained encoder object, complete metadata, and pinned benchmark worker
rebuild byte for byte. All 531 syntax unit tests pass. Review the three source
files under `src/syntax/src/document/parser` and the
[page-size, parser, and complete build evidence](evidence/parser-journal-pages/report.json).
All build comparisons include final linking with compilers, fixed runtimes, and
adapters prebuilt. See the README for phase timings and exact rebuild commands.

## Fresh owner reference counts

The new pass folds 20 reference-count loads from direct, checked allocations
that have known nonempty storage. It preserves the writes and checked paths.
Review `hosts/gpu/src/native_artifact/program/compact/owner_counts.rs` and its
tests. The five native test groups cover aliasing stores, unknown calls, reused
backings, loop changes, allocation failures, and joins.

Its encoder is **72,344 B (1.25388× Roc, 1.95102× libdeflate)** with unchanged
5,296-byte peak stack. Both full peer orders record **81/81 Roc and 61/81
libdeflate** for candidate and control. The full matched runs bound 77 cases at
0.5% resolution. The [report and raw observations](evidence/fresh-owner-counts/report.json)
include longer runs for the remaining four cases. The duplicate controls vary
beyond 0.5% on three cases, leaving the pass opt-in. The matched build takes
6.24649 seconds, effectively flat against 6.25214 seconds for the control and
12.96053× libdeflate. Use `rebuild.py --variant fresh-owner-counts` to reproduce
the candidate with exact object and metadata checks.

The 72,024-byte cleanup combination still has two sustained slowdowns in the
focused screen. Its exact rebuild variant is `fresh-owner-counts-cleanup`.

## Earlier experiments

`evidence/size-screens` contains all 24 static screens: ten additional shared-region
families, eight individual compiler policies, and six combinations. These use
the compiler source already published in this branch. No production source or
retained recipe was changed for this follow-up.

| Configuration | Encoder text | × libdeflate | Root frame | Runtime qualification |
| --- | ---: | ---: | ---: | --- |
| Retained baseline | 72,520 B | 1.95577 | 3,744 B | Historical full 81/61 |
| Range-derived owner proofs | 72,392 B | 1.95232 | 3,744 B | Full 81 Roc; C 61/60; not promoted |
| Owner proofs, inline only helper 4879 | 72,168 B | 1.94628 | 3,680 B | Rejected after two focused orders |
| Owner proofs and final cleanup | 72,136 B | 1.94542 | 3,792 B | Full 81/61 in both orders; rejected on five cases |
| Owner proofs, final cleanup, cold tails | 71,768 B | 1.93549 | 3,792 B | Pending |

The narrower proof removes 128 bytes, passes correctness and checked-failure
controls, and keeps the same peer wins as its simultaneous control. Both control
and candidate have 61 then 60 C wins in the full orders; the close case wins in
both longer follow-ups. Tiny-input repeatability remains unresolved, so the
candidate is not promoted. Its matched complete build is 6.26348 seconds versus
6.24346 for the control, with no demonstrated build-time improvement.

The configuration that removes 352 bytes and reduces the root frame by 64 bytes
loses C wins and consistently slows several short cases. It is rejected despite
passing correctness. The smallest static candidate removes 752 bytes but
increases the root frame by 48 bytes; it has not been linked or runtime-qualified.
Do not attach the baseline's win counts to these candidates. The root frame is
not complete stack high-water, and metadata calls are not final relocated calls.
See [the complete follow-up](evidence/owner-proof-runtime/report.json) for raw
paired data, repeated same-worker controls, and every adverse observation.

The owner-proof and final cleanup combination keeps 81 Roc and 61 libdeflate
wins in both full orders. Five cases show a slowdown above the 0.5% resolution
and peak stack grows from 5,296 to 5,344 bytes, so the result is rejected. See
[the full report](evidence/owner-cleanup-full/report.json) and
[the earlier focused report](evidence/owner-cleanup-focus/report.json) for raw
measurements and the exact candidate identity. Its static reproduction case is
`combinations--owners-cleanup`. `replay.py` recomputes wins from both sets of runs.

All ten additional shared-region screens were at least as large as the baseline.
The stack-pair census found only 156 bytes potentially removable by pairing
adjacent stack operations. Root IR, region inventories, negative results, raw
emission logs, complete metadata, configuration differences, and hashes are
included so the next agent can inspect these results without recreating local
scratch directories. Raw receipts preserve original paths as provenance.

After running `rebuild.py` as described in the README, reproduce the static
screens from this checkout with:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/size_screens.py \
  /tmp/native-size-screens \
  --emitter /tmp/native-baseline/compiler/release/emit_compact_size
```

Use `--list` to list configurations or `--case combinations--owners-inline-4879`
to select one. The script checks the exact object and metadata hashes, text size,
root frame, and metadata call count for every selected case. It does not qualify
runtime performance. The recorded single-emission durations are not matched
source-to-executable benchmarks.

## Review priorities

1. Review the native compiler changes against #973, particularly checked-region
   boundaries, live arguments, owner proofs, direct runtime declarations in
   compilation workers, and the deferred publication order. Start in
   `hosts/gpu/src/native_artifact/program/compact`, including `compile_jobs.rs`.
2. Review the vendored ARM64 backend's branch, relocation, call, trap, and unwind
   updates for cold-tail sharing. The pass remains opt-in. Its existing 15-case
   results include adverse tiny-input estimates; it is not a new full 81/61 result.
3. Inspect `evidence/integration` and `evidence/compiler-allocator` before reusing
   performance claims. Keep object identity, linked-worker identity, correctness,
   and runtime qualification distinct.
4. Resolve the remaining CI failures and finish native-policy qualification.
   The PR remains a draft; whole-stack green CI is not claimed. The latest
   pre-publication CI snapshot is included with the size-screen evidence.
5. Before promoting a smaller candidate, run correctness and checked-error
   controls, both complete peer timing orders, and per-case uncertainty checks
   with sufficient measurement duration for tiny inputs. Preserve the 81/61
   floors and investigate individual regressions, even if total wins hold.

The optional cold-tail candidate in the main README and the 24 new static screens
are separate experiments. All smaller candidates remain opt-in pending the
required qualification.
