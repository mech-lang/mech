# Current geometry in native enum-state loops

Base: `712a930950aed48fe2360eedd81a731449ff512e` (clean exact-head review). Native IR/work/ABI/bytecode: **213/107/39/14**. This change affects native lowering only. Both compression sources, source allocation sites, tables, capacities, effort, checked arithmetic, and timing boundaries are preserved.

The actual match-extension state machines each contain 424 source blocks. Their counters live in different enum payloads. The existing captured-window analysis requires an affine private-frame carrier and an acyclic visit within its optional 256-site value-analysis allowance. It therefore cannot recover the old loop specialization from these machines. The original guarded wide loads still execute; this gap is the repeated geometry guard and retained scalar fallback, not the absence of every wide load.

The new version uses a smaller fact with an independent proof: the current immutable buffer has one row or one column. The existing numeric proof establishes that every child alternative keeps that buffer's root identity, view and owner; the geometry gate checks the current view after the canonical initializer has copied the child's current arguments. A successful gate selects a cloned native child using the maintained scalar, aggregate and packed-read emitters. Refusal enters the original checked child. A cold/unknown edge leaves the selected version and cannot carry its fact into an ordinary continuation.

Only private scalar enum-state children without an existing window version qualify. Only read-only, non-growing numeric views qualify. No source identifiers, field names, site numbers, compression constants, or input-specific tests participate. Ordinary recursive/record children retain their routes.

Geometry does not grant an index range, a tag, a trip count, an arithmetic result, an owner, or funding. Every fully lowered pack still needs the existing freshly executed source window proof and a complete confined expression. Source length/subtraction/comparison checks and their selected paths execute in their original order. Unrecognized expressions retain their guarded scalar fallback. Every original per-visit child debit, exact fuel/error snapshot, publication and cleanup remains in use; no prepayment or permission cache is added.

The affine analyzer still receives its original ordered scope and unchanged analysis allowance. Geometry neither needs acyclicity nor a trip bound: its view is immutable for the enclosing numeric phase. When optional local value analysis declines, the geometry version keeps checked operations rather than inventing replacements. This removes the accidental dependency between an invariant shape and a counter proof without increasing execution allowances or weakening rejection paths.

The image IR is advanced to213 because generated code and optimization selection change. The maintained image loader rederives the same trusted plan/proof; bytecode14, ABI39 and work policy107 remain.

The regression is generated from the maintained byte-pack source. Its two-state child exceeds256 source blocks and performs two windowed expressions per state. Exactly two expressions have the existing source-window proof; the separately checked `at+8` expressions remain guarded. Tests cover original/decoded artifacts and restored images, changing row/column/matrix/empty/short geometry, unselected maximum offsets, arithmetic failures, an independent column-order byte oracle, aliases, unchanged publication, cleanup, retry and six finite resource dimensions.

The clean parent fails the new packed-state witness. The first prototype passed a smaller witness but did not reach the full 424-block machines; that generated image and failed selectivity are preserved. The corrected implementation selects all three real machines and fully lowers12 packed-read occurrences. Static counts include variants and are not dynamic visit or retired-instruction counts. Generated state-source code decreases6,781,820→6,761,964 bytes (−19,856 bytes); its machine entry stack remains5,536 bytes. Existing-source native text remains byte-identical (8,152,568 bytes with alignment, SHA-256 `3482985120f0e0cc576667fac72bcf27324ff4dc7407433e97a81bdb6503006f`).


## Results

Both compiler-change comparisons pass all20 unchanged1.05 gates. The existing source has byte-identical native text; its elapsed control is+0.533317% geometrically, including repeated+4.924713% and periodic+3.948521%, so no existing-source speedup is claimed. The state source improves1.769709% geometrically, with22.312815% less repeated-input time and12.808121% less periodic time. Its natural-text input takes4.840953% more time and5.276477% more instructions. Keep this tradeoff explicit.

The state source still fails seven adoption gates and remains19.264007% slower geometrically than the retained source. The existing source is retained.

Eight balanced blocks, all four executable/source combinations at every block position twice:7,936 primary /8,096 total complete calls, plus256 fresh reference repetitions per input. Complete evaluate_into only; image startup, preparation, validation and dropping are excluded. Input/source hashes, allocator, real buffers, source allocation sites and search effort remain pinned. All cases agree with exact raw bytes and independent inflation, preserve aliases/prefix/capacity, and finish with zero managed bytes/nodes. Three reference medians remain below resolution.

| Input | Existing / Roc | State before / Roc | State after / Roc |
|---|---:|---:|---:|
| reymont-prefix-32769 | 3.343891× | 3.765244× | 3.947518× |
| random-32769 | 4.775729× | 5.586406× | 5.618974× |
| 16384-distribution-change | 6.198885× | 6.757119× | 6.806140× |
| repeated-32769 | 3.148675× | 12.020990× | 9.338768× |
| 33027-periodic-window | 3.654774× | 22.944172× | 20.005454× |
| eight-literals | 4.466182× | 4.422801× | 4.435860× |
| reymont-prefix-16384 | 4.051924× | 4.473333× | 4.589280× |
| empty | — | — | — |
| one-stored | — | — | — |
| seven-stored-prefix | — | — | — |
| eight-overlap | 4.387195× | 4.367523× | 4.376960× |
| 320-overlap | 4.259143× | 4.479308× | 4.436367× |
| eight-literals-prefix | 4.616725× | 4.503775× | 4.595312× |
| 128-biased-dynamic | 3.857339× | 3.836002× | 3.821943× |
| 32-high-entropy-stored | 4.231142× | 4.243421× | 4.179314× |
| random-64 | 4.173731× | 4.150183× | 4.149757× |
| random-256 | 4.306651× | 4.284247× | 4.286315× |
| random-1024 | 3.918866× | 3.992505× | 3.996149× |
| repeated-1024 | 4.136321× | 5.140011× | 4.833075× |
| reymont-prefix-1024 | 4.573009× | 4.642619× | 4.673591× |

Separate counter cohorts show repeated instructions339,940,612→236,670,665 (−30.378820%) and periodic564,375,976.5→460,591,155 (−18.389305%). All five measured allocation/requested-byte/incremental-usable-peak pairs are identical; no allocation saving is claimed.

A fresh periodic state-source wall-stack sample puts45.69% of exclusive weights in generated code and53.28% in host code below generated entry. Leading host-exclusive weights are Invocation::visit274/2055, numeric_source_prepaid133, charge_compute_with_limit71 and visit_from_entry63. These are separate stack samples, not retired-instruction attribution or callback counts. The full packet preserves both samples, counter/cycle observations and contrary timing blocks as aggregates/receipts; raw local traces are excluded.

The next bounded experiment can share immutable view geometry at native-region entry rather than retest it at every child activation. Current emitted code and the natural-text instruction regression justify an experiment, not a predicted gain. Source-visit/accounting callbacks, tag dispatch and enum-state trip/range proofs remain separate work.

## Validation and scope

673 native tests, all256 scheduled names, three new tests, unchanged236 components /eight compositions /one real-capacity control,1724 native match controls, resident/state agreement,15 architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings denied pass. The exact final regression fails on the clean parent with zero selected geometry scopes. The same seven inherited native exclusions and old-source resident default-work denials remain disclosed. No expected values or allowances increase.

GPU-only optimized3 builds use R196 Engine/Compute/Runtime/thin Stdlib and frozen Core/Math/Syntax. Tests use optimized1. Full Cargo/all-features/dependency and registry-project rebuilds, multi-megabyte inputs and separate workloads are skipped. Linked-image startup takes35.97–37.54seconds per worker, explicitly outside hot-call timing. Ordinary hosted CI is cancelled under the standing instruction; #973 remains draft and exact-head review is requested.

[Immutable evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r197-state-geometry-928564035718.tar.gz) — SHA-256 `92856403571868f11e9361ba24727265201d5ab36238a31fee04bb7f7aa6b8d5`, 2,409,653 bytes / 523 hash-listed files. Run heap-summary.py then analyze.py to reconstruct results without executing a compiler or compressor. The initial-order cohort and corrected harness/setup failures remain preserved.
