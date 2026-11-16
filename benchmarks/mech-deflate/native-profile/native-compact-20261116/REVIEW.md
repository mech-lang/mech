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

The frontend allocator change is published in `21757538`. The matched complete
source-to-executable comparison is **6.24081 seconds**, or **2.09436× Roc and
13.00485× libdeflate**. This includes final linking, with compilers, fixed runtimes,
and adapters prebuilt. The change improves the same-input control by 8.83%, with
all ten Mech outputs byte-identical. See the main README for the raw evidence and
the distinction between this benchmark and compiler construction.

## Latest experiments, not promoted

`evidence/size-screens` contains all 24 static screens: ten additional shared-region
families, eight individual compiler policies, and six combinations. These use
the compiler source already published in this branch. No production source or
retained recipe was changed for this follow-up.

| Configuration | Encoder text | × libdeflate | Root frame | Runtime qualification |
| --- | ---: | ---: | ---: | --- |
| Retained baseline | 72,520 B | 1.95577 | 3,744 B | Historical full 81/61 |
| Owner proofs, inline only helper 4879 | 72,168 B | 1.94628 | 3,680 B | Pending |
| Owner proofs, final cleanup, cold tails | 71,768 B | 1.93549 | 3,792 B | Pending |

The first candidate removes 352 bytes and reduces the root frame by 64 bytes.
The smallest candidate removes 752 bytes but increases the root frame by 48
bytes. Neither has been linked, correctness-tested, or timed against the peers.
Do not attach the baseline's win counts to either candidate. The root frame is
not complete stack high-water, and the compiler's metadata call count is not a
count of the final relocated calls.

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
