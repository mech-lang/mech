# Native scalar enum-state lowering (R196)

The compiler now stores enum states without payloads in native words and preserves their current tag/payload at exact child-fuel exits. The existing compression source is retained. Its generated instructions are byte-identical, and all twenty original timing gates pass. The state-machine source remains a qualified but slower acceptance witness.

Source base: `e35a8a9a44ba488d9f6f5fdf6aebc68cdc6b9d15`; previous retained compiler: `cadaabefcf7dfb64b1b32a724e53c300f7c1fca1`. **IR/work/ABI/bytecode: 212/107/39/14.** The public packet pins every changed compiler/test file by SHA-256 and Git blob ID.

## Compiler change

* An absent enum payload uses zero payload words plus the tag. The existing maximum-width padding and connected escape analysis still determine private storage. Canonical values and unsupported payload kinds keep their current representation.
* Exact child-fuel continuation analysis recognizes a live private enum as scalar snapshot lanes. The shared snapshot emitter already saves those lanes; it does not require a buffer owner.
* Private scalar enum-state children try the complete, per-visit-funded native phase before a partial ordinary child. Ordinary recursive and record children retain their previous routes. Eligibility uses trusted schema/storage facts, with no source names, field names, timing inputs or site-specific rules.

The canonical Plan, selected factories, shared scalar/array emitter, current resource accounts, runtime publication/cleanup and independent image proof reconstruction remain in use. Native image IR212 distinguishes the layout/proof change. Source-derived bytecode14 and ABI39 remain.

## Measured results

Each final matrix has eight alternating matched blocks per input and 256 fresh reference repetitions per input: **3,968 timed / 4,048 total complete calls**. Build/restore, seed/clear/prefix preparation, validation and dropping are outside `evaluate_into`. Allocator, real buffers, source allocation sites, tables, depth300/nice258, EOF behavior and timing boundaries stay pinned.

**Compiler-only, existing source:** all20 gates pass; fourteen faster/six slower medians; geometric change −0.165016%. The Mach-O text is byte-identical (8,152,568 B with alignment; 8,152,248 emitted code B), and stack remains16,672 B. The fluctuations are not claimed as a compiler speedup.

| Input | Before / Roc | Retained compiler / Roc |
|---|---:|---:|
| Natural text 32,769 B | 3.343259× | 3.371926× |
| Random 32,769 B | 4.771033× | 4.757907× |
| Distribution change 16,384 B | 6.242475× | 6.236508× |
| Repeated 32,769 B | 2.981417× | 2.972509× |
| Periodic 33,027 B | 3.501140× | 3.521470× |
| Eight literals | 4.547953× | 4.528874× |
| Natural text 16,384 B | 4.046147× | 4.012777× |

**Same state-machine source, separate matched cohort:** all20 complete inputs agree exactly with the reference and independent decompression, but only12/20 timing gates pass. Eight adverse gates prevent source adoption; geometric elapsed increases21.850993%. Natural32 moves3.325419×→3.747200×; repeated32 moves2.986418×→12.008249×; periodic33 moves3.516583×→22.972440×. Eight literals move4.693653×→4.636123× in this separate cohort. Reference denominators differ between cohorts; use each matched pair. Three stored-case reference medians are zero and their reference ratios remain unresolved.

The previously pathological state-source case completes in about16 ms after the compiler correction, versus19.8 s in R195. That historical pilot comparison is not a matched percentage claim. The faster existing source is preserved.

## Remaining cost

Separate instruction/heap cohorts use16 interior observations per side for five inputs. Repeated32 instructions rise70,996,927.5→339,935,759.5 (**4.788035×**); periodic33 rises76,720,258→564,343,011 (**7.355854×**). Their requests change3135→3147 and3679→3652. Natural32 instructions rise209,003,213→241,228,703 while requests change6449→6461. These measured source differences chiefly increase instruction work; they are not a large allocation-count increase. Requested bytes and incremental usable heap are slightly lower, with exact values in the packet.

In the same outer native loop, fully lowered packed-read occurrences fall18→0 and proved child-entry checks30→0. Guarded wide reads still exist. The existing full-lowering predicate requires a source window and a flat-array loop version; the state form loses those versions. Static counts include variants and are not dynamic visits or per-function instruction attribution. Restoring current captured buffer/limit facts through enum-state transitions is the next bounded compiler hypothesis. No fresh function sample is claimed.

## Validation and disposition

* **670 native tests pass**; all **253 scheduled CI names** execute in that passing run. The same seven inherited exclusions remain, with exact names in receipts.
* Two new maintained tests cover tag-only/nonzero terminal states, tag/payload states, original/decoded artifacts, restored images, Boolean captures changing between parent visits, unselected calls, assertion-before-load/division, bounds rejection, live aliases/prior publication, an actual child-header fuel exit, cleanup and retry.
* Unchanged **236 component / eight composition / one real-capacity** tests pass without expected-value or allowance increases. All **1,724 native match controls** pass the independent byte oracle. All862 candidate resident controls agree; the old source retains71 disclosed resident default-work denials per artifact form. Final pure-state controls also cover overflow and finite rejection/retry against resident execution.
* Fifteen architecture/catalog checks,132 workflow Python tests, runtime-only, formatting and warnings denied pass. Engine/Compute/Runtime/thin Stdlib are rebuilt against frozen Core/Math/Syntax. The explicit Runtime profile excludes unused watcher/notify/ignore dependencies; this is not a fresh Cargo all-features build.
* The first global selection reorder failed four existing transport-route tests. Narrowing selection to scalar enum-state children and export support to enum states restores all four tests unchanged. A broader record-export prototype remains preserved and unmeasured; it is outside this patch.
* All final linked cohorts clean up to zero managed bytes/nodes. The initial12.393-second nullary-only screen was intentionally stopped after a complete call; no final cleanup receipt is claimed for that stopped worker. Corrected setup/test failures and exact failed commands remain in the packet.
* Full dependency/all-features rebuilds, registry-generated-project reruns, multi-megabyte timing, a new function-sampling study and the separate workload suite are skipped. Hosted CI is not awaited; ordinary exact-head CI is cancelled under the standing instruction, review automation remains, and #973 stays draft.

Review this bounded state-machine correction before expanding the next compiler change. Existing vector/logical-selection/dynamic-state payload gaps remain separate reproducers. No compression algorithm, benchmark source, table, search limit, allocation site or timing-boundary change is adopted.

[Full evidence and all twenty rows in each cohort](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r196-native-enum-state-94598fa9fa95.tar.gz) — SHA-256 `94598fa9fa9501b2852928d42011a9f1d4ab35d5e82d6b95cb90ce5d6f891fcd`, 6,000,417 bytes / 880 hash-listed files. `heap-summary.py` and `analyze.py` reconstruct the numbers from receipts without compiling or running a compressor. Native binaries, bytecode, executable descriptors and raw traces are excluded.
