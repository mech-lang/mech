# One substantial checked codebook phase

**Experimental, not selected.** The selected M1 encoder remains the fully
qualified **63,804 B** `m1-preserved-root-registers` recipe. This experiment
emits **64,444 B**, 640 B more. Its object is `4f6694cf…`; it is a different
build from the earlier qualified 64,444 B literal-frame recipe.

The original Mech source and canonical bytecode are unchanged. One checked
codebook region now compiles separately, using the existing boundary admission,
local IR preparation, input/output transport, error origins and full SSA checks.
This is a late split: the compiler still constructs the global source/native
plan. It does not complete the planned small-controller architecture.

| Apple M1 | Selected control | Checked-phase experiment |
| --- | ---: | ---: |
| Complete encoder text | 63,804 B | 64,444 B |
| Checked controller | 46,788 B | 41,336 B |
| New phase | — | 6,008 B |
| Root native frame | 3,296 B | 2,880 B |
| New phase frame | — | 496 B |
| Measured peak active native stack | 4,848 B | 4,688 B |

The exact linked workers pass 162 matching output/decompression/borrowed-alias
observations and all six allocation-failure cases, with cleanup and unchanged
source origins checked. The stack probe covers both samples of all 81 cases,
including nested calls. `linked-result.json`, `linked-checked-failures.json`,
`linked-stack.json` and `linked-proof-sources.tar.gz` record those checks.

**Runtime win counts are unmeasured for this experiment.**
Focused JIT tests are building; runtime measurements remain pending. Do not inherit the selected control's 81/81 Roc and 62/81 libdeflate
wins, or the older literal-frame recipe's results, for this different worker.

## Matched fresh builds

Eight balanced fresh samples per actor, with excluded warmups, use one compiler
and fixed support prebuilt for every actor. Every measured build includes final
linking, and each regenerated encoder and full metadata is checked against its
pinned identity. All current-job compiler processes were stopped during the
measurement window and automatically resumed afterward; see
`timing-isolation.json`.

| Same-session build | Source through final link | Through strip | Encoder text |
| --- | ---: | ---: | ---: |
| Selected control | 2.245295 s | 2.256457 s | 63,804 B |
| Checked-phase experiment | 2.231624 s | 2.242875 s | 64,444 B |
| Roc | 2.997875 s | 3.009078 s | 57,696 B |
| libdeflate | 0.483540 s | 0.494262 s | 37,080 B |

The paired saving is **14.000 ms**, 7/8 faster, with a per-cohort 90% interval
of **12.710–18.280 ms** saved (20,000 bootstrap resamples, seed 9951119).
The median paired ratio is 0.99377 versus the control, 0.74597 versus Roc and
4.62165 versus C. Stripped deployment remains 276,072 B. This is a measured
build improvement with a 640 B encoder cost, not a size win or runtime
promotion. The earlier qualified 2.239 s measurement is a different cohort.

Native phase medians are 1.238337 s for the control and 1.222933 s for the
experiment. Source CPU medians are 2.273029 and 2.259801 s. Source RSS medians
are 392,757,248 and 387,956,736 B. Catalog phase values are join wait; catalog
work still executes. Full per-trial data, identities and phase measurements
are in `build-plan.json`, `build-comparison.json.gz` and `build-summary.json`.

## What the experiment found

The initial 2,236-block split grew to 66,748 B. Fresh region IR had lost the
root's local backend policies and late cleanup. Restoring them folded 22 reads,
shared 38 error returns and recovered 1,056 B.

The narrower boundary, native blocks `4069:4954`, has 1,115 blocks and 1,946 IR
instructions before extraction. Admission finds 18 input words and six normal
continuation words; passthrough leaves a 15-word input packet and two register
outputs. The call encloses codebook work rather than a character/scorer inner
iteration. The compiler independently proves the complete graph boundary;
`structural-candidates.json` is only a discovery aid.

The extracted phase missed argument setup shared by 23 original heap calls.
Its own invariant entry heap capture now feeds uniquely named, frame-local
stubs. Both ordinary loads and the low/high halves of paired entry loads are
admitted. Each stub is defined using its own compiled frame; the root's frame
is never reused. The stub bodies, provider ABI, allocation/refusal/cleanup
policy and frame bounds checks remain the existing implementation. In this
case the phase saves 112 B but its private stubs add 128 B, a net 16 B cost.

Two formerly inlined numeric helpers also reappeared as standalone bodies.
Checked phases now use the prepared Object's compilation path and the existing
private helper selection. That recovers 80 B net in this case. The current
recipe's two selected helper call sites both move into this phase; this is not
a blanket no-inline or always-inline policy. A preselection attempt with no
reference to any recorded eligible helper now skips the unused clone and
optimization, while keeping normal compilation and verification.

`variants.json` and per-variant receipts preserve intermediate and adverse
results. `symbol-deltas.json` in the final variant includes alignment between
symbols; the metadata records the separate raw body sizes. None of the single
source-emission phase logs establishes a complete-build timing effect.

## Source replay

On this PR's source, first rebuild the selected control:

```sh
python3 -B rebuild.py /tmp/phase-control --variant m1-preserved-root-registers
python3 -B evidence/encoder-size-gap/checked-codebook-phase/replay.py \
  /tmp/phase-control /tmp/phase-experiment
```

This verifies the exact experimental object, complete metadata and canonical
bytecode. It does not relink or measure runtime. The recorded linked checks
are separate evidence, and a new host-support worker requires its own receipt.
The as-run compiler is `4e1d5afe0ee1791bfbf4d624ff05a7665f90c5a64e3fb66a6fc08ccf5e4114cb`,
from source base `d8faf1969b` plus the recorded five-file native compiler patch.
`compiler-source.patch.gz` is historical reproduction evidence; this PR contains
the implementation as live source.

For discovery replay, extract `region-boundaries/after-regions.tsv` from the
neighboring `preserved-root-registers/qualified-root-diagnostics.tar.gz`, then
run `analyze-controller-regions.py INVENTORY --output candidates.json`.
The compiler's admission remains mandatory after this heuristic ranking.
