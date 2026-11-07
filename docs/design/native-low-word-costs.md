# Scalar low-word reduction costs — R188

Reject the current variant: the real scalar path improves16 to12 instructions and natural32 improves0.807595% in8/8 paired blocks, but aggregate elapsed improves only0.070110%;17 instruction medians,12 cycle medians and11 scalar-control medians rise. Code grows1236B and entry stack352B with unchanged heap. Preserve the complete experiment and return production to qualified R186; do not discard or repeat any sample.

The candidate is rejected. Production is byte-for-byte R186/IR206. The real local instruction reduction is preserved as evidence, alongside every adverse observation; no sample is discarded or repeated.

## Implementation and limits

The level-12 optimal-path minimum carries cost in the high half and index+1 in the low half of one private U64. Zero retains the incoming item. Current nonempty entry and stop≤U32MAX prove that each visited index+1 is nonzero and fits U32. Strict incoming ties and earliest winning index are preserved, including the vector-prefix sentinel and scalar remainder.

This refinement evaluates the scalar cost in an I64 register with only its low word meaningful. Projection modulo 2^32 commutes with every admitted operation: addition, AND, OR and XOR. Shifting by32 during packing discards the unconstrained high word. Captured words are read once, with their existing I32 vector form and original scalar register retained. Scalar reads remain exactly four physical bytes for either four- or eight-byte backing. Vector cost emission and all entry/window/funding/refusal/terminal proofs stay unchanged.

Source-mapped inspection of the actual Object confirms **16 →12 instructions** on the continuing scalar path; the rejected R187 variant has13 and the pinned reference has11. The redundant zero-extension move is gone. This is a static path count, not a dynamic visit census or whole-call prediction. Fresh Clang and pinned Rust scalar diagnostics share one comparison between two selections; they never link into the compressor.

Candidate code **8,153,276 →8,154,512B (+1,236)** and reported entry stack **16,480 →16,832B (+352)**. Four numeric reports change only maximum_source_instructions167→182; the existing source envelope and every allowance remain unchanged. Candidate IR/work/ABI/MBC **208/107/39/14**; restored production **206/107/39/14**.

## All 20 complete-call timings

All original1.05 gates pass: **10 faster/10 slower medians**, geometric elapsed **-0.070110%**. Natural32 improves in all eight blocks; its0.807595% gain is real within this cohort. Random32 has four improving/four adverse blocks, periodic four/four, eight-literal three/five. The complete predeclared cohort contains3,968 primary/4,048 total calls, maximum33,027B. Three zero reference medians remain unresolved.

| Input | Before / Roc | Rejected candidate / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.603628× | 3.574525× | -0.807595% | 8/0 |
| reymont-prefix-16384 | 4.345024× | 4.310893× | -0.785525% | 5/3 |
| random-32769 | 4.767305× | 4.781002× | +0.287310% | 4/4 |
| 16384-distribution-change | 6.287962× | 6.300231× | +0.195113% | 4/4 |
| repeated-32769 | 2.976170× | 2.978807× | +0.088619% | 5/3 |
| 33027-periodic-window | 3.462555× | 3.486903× | +0.703187% | 4/4 |
| eight-literals | 4.416522× | 4.427971× | +0.259227% | 3/5 |
| empty | unresolved | unresolved | -0.411660% | 7/1 |
| one-stored | unresolved | unresolved | -0.587452% | 7/1 |
| seven-stored-prefix | unresolved | unresolved | -0.467864% | 7/1 |
| eight-overlap | 4.367656× | 4.374126× | +0.148122% | 3/5 |
| 320-overlap | 4.130156× | 4.152801× | +0.548296% | 4/4 |
| eight-literals-prefix | 4.507212× | 4.502621× | -0.101857% | 5/3 |
| 128-biased-dynamic | 3.799059× | 3.792463× | -0.173636% | 4/4 |
| 32-high-entropy-stored | 4.161770× | 4.142104× | -0.472555% | 4/4 |
| random-64 | 4.209790× | 4.221444× | +0.276827% | 5/3 |
| random-256 | 4.246515× | 4.260735× | +0.334866% | 4/4 |
| random-1024 | 3.843169× | 3.840438× | -0.071065% | 4/4 |
| repeated-1024 | 4.095201× | 4.097652× | +0.059837% | 5/3 |
| reymont-prefix-1024 | 4.688447× | 4.669382× | -0.406641% | 6/2 |

Each pair uses one fresh pinned reference. Independent rounds are not compounded. Exact raw bytes, independent inflation, logical work, input aliases, output prefix/capacity/reuse and zero final managed bytes/nodes agree. Logical work is not a hardware instruction count.

## Separate counters, memory and scalar controls

**Three lower/17 higher instruction medians;12 cycle medians higher.** Natural32 instructions fall0.849426% while its separate cycle median rises1.557448%. These cohorts remain separate from primary elapsed and timer samples.

| Input | Instructions before | Instructions candidate | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 215,887,172.0 | 214,053,370.5 | -0.849426% | +1.557448% |
| reymont-prefix-16384 | 122,793,362.0 | 121,808,227.0 | -0.802271% | +1.653740% |
| random-32769 | 71,374,991.5 | 71,473,130.5 | +0.137498% | -0.280133% |
| 16384-distribution-change | 65,604,378.0 | 65,626,413.0 | +0.033588% | +0.129400% |
| repeated-32769 | 71,099,211.5 | 71,129,848.5 | +0.043090% | -0.308043% |
| 33027-periodic-window | 76,779,294.0 | 76,806,267.0 | +0.035131% | +0.972426% |
| eight-literals | 12,449,945.0 | 12,450,525.0 | +0.004659% | -0.464747% |

Allocation requests, requested bytes, reallocations, incremental peak and before/after/peak usable-live heap medians are **exactly equal in all20 cases**. No streaming, initialized-copy-volume, RSS or heap gain is claimed.

All12 scalar controls pass the unchanged latency/allocation/no-slope gates, but **11 medians are adverse**, ratio range **0.999867459–1.011822082**. This broader tradeoff, the larger stack and17 higher instruction medians cause rejection; the gain is not rejected merely for being small.

## A larger target exposed by the new profiles

Five exact-candidate profiles complete **17,838 correct calls**, every recorder and per-case/final cleanup zero. The exact-current source-location build repeats the entire Object, descriptor, metadata, source and bytecode byte-for-byte. It records115,836 source tags and265,573 spans. CPU timer weights are not retired-instruction counts; outside-main work includes other generated helpers, useful runtime algorithms and harness work.

The previously unattributed native PCs have now been decoded against both actual Objects. In this subset alone, stack loads/stores account for the following percentages of all running timer samples:

| Input | Retained compiler profile | Rejected candidate profile |
|---|---:|---:|
| reymont-prefix-32769 | 19.916636% | 19.038748% |
| random-32769 | 23.319054% | 24.005973% |
| repeated-32769 | 19.325598% | 18.239463% |
| 16384-distribution-change | 15.325742% | 15.611578% |
| eight-literals | 1.441701% | 1.554960% |

These are separate sample cohorts, not measured before/after savings. Stack traffic includes necessary program homes and possible register-allocation spills; that distinction and removability are not established. The next bounded investigation will map the hottest physical stack accesses back to their homes and live ranges before changing lowering. Existing nontrapping-home/SSA/ownership improvements are not being reproposed. Branch-only routing is also recorded, with no claim that every branch is removable.

## Validation, failures and provenance

**11 focused/665 candidate native tests pass; all248 CI-selected names actually execute**, with the same seven inherited exclusions. The new production-helper controls execute64,800 low-word projection comparisons against ten independent U32 expressions plus12,288 choice comparisons: full-width captures/costs, wrapping, ties, sentinels, current visits, both storage widths, near-limit indices, empty visits and unchanged inputs. Maintained source tests cover original/decoded/current/restored windows, vector/scalar boundaries, finite denial, prior publication and retry. Unchanged236 component/eight composition/one real-capacity-path tests pass; all15 architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings-as-errors pass. No expected values or allowances increase.

Initial builder-signature, test-layout, formatting and copied harness-identity errors are fixed; their failed receipts remain. A full-suite receipt-path orchestration error is corrected by rebuilding and rerunning the final665-test suite. Disk exhaustion during three auxiliary builds is resolved by removing339 obsolete reproducible native build files while preserving all source, logs, archives, fixtures, frozen dependencies and the measured baseline; about29GiB was recovered. One redundant main image producer ended with SIGKILL and no diagnostic; its cause remains unproved. It is not relabeled as passed. The independent current-source generation succeeds and repeats every image byte, and both actual executables contain their exact current descriptor once and define the expected identity symbol. Thus the terminated duplicate does not invalidate the measured image.

Public parent `feb2149d5c212ca0b284cc19e1d7ff0932c1b5f5`; rejected measured source `7674d1f4b51c3a3997b6418588b84083413441d9`; unchanged production source `582c8b00cf86df926f21845573222c3cf9ba117f`. Candidate Object `bc2c4b41554e8e78b54a25c78df4b39a676b594e9311a1312e9bc447f19fd48f`; worker `785a148711b56620bf66495292f07c13b80263c6ee84141bc1cd08b8a650f0e1`; descriptor `30a4e71b8a6924d7027abfc4d7274609f75cfb5a69135e0aa6b9dfc43e707088`. All seven candidate source files, exact patch and raw source-commit data are retained, independently of the final evidence-only commit. Restored production reuses the already-qualified R186663/246 evidence; candidate665/248 counts are not relabeled as fresh production results.

Source/MBC/algorithm/tables/depth300/nice258/EOF/ties/source allocation sites, real262144-entry U16 tree/1500513-word U32 cache/305257-entry U64 path capacity, output13 clear/reuse, input aliases, pinned mimalloc2.1.7 and full evaluate_into timing remain unchanged. FrozenR145 dependencies; no math-library rebuild. No compiler or tests overlap measurement. Only existing draft#973 is updated. Ordinary exact-head CI is cancelled by instruction and review automation preserved. Fresh all-feature/dependency builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. Parity remains incomplete.

Run checkpoint.py on chronologically verified evidence to reconstruct REPORT.md/results.json numerically without compiling or compressing. analyze-unattributed.py additionally reproduces the stack/branch classification when the exact local Objects are available. Public replay verifies all archived versions, receipts and derived output hashes.
