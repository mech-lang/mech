# Selected native ranges — R185

Retain selected-edge SSA range propagation: all original gates pass; sixteen elapsed medians improve, with four contrary cases retained. Whole-call instruction counts rise in the five larger cases despite lower cycle medians there; all independent cohorts remain explicit. No parity claim.

The compiler propagates facts from a unique selected CFG edge only within its dominator subtree. Facts refer to immutable SSA words, not memory, captures or ownership permissions. Lossless Boolean widening, true conjunctions, false disjunctions, unsigned constant comparisons and safely inverted modular intervals are supported. Shared branch destinations, joins with other predecessors, entry blocks, unsupported exception edges and fresh loaded values keep their checks. Each loop visit uses its current SSA arguments. Source effects, checked errors, work debits and atomic publication remain. No source/allocator/runtime ABI change.

## Actual generated code

The complete compressor removes **8,721 proved branch sites**. This is a static count, not an executed-call count. Generated text **8,461,948 → 8,153,724 B** (−308,224); stack **16,976 → 16,912 B** (−64). All earlier numeric recipes, selections and window proofs remain equal. Dead IR instructions removed:43,492; loads24,298; actual block parameters0. IR/work/ABI/MBC **205/107/39/14**. The ordinary single backend data pass is preserved.

The concrete source requirement is the cached-match-length check in3..258 followed by checked subtraction of2, at pinned source line709. Clang and Rust both eliminate the second guard in equivalent diagnostic source; neither diagnostic helper is linked into the compressor. An actual parsed/canonical/original-decoded array-bearing recursive source witness activates four folds (parent zero), agrees with the resident oracle, and preserves exact native work/results/errors/aliases/publication/cleanup/retry. The parent fails the new coverage assertion with expected exit101; the candidate passes. The scalar-only source control preserves semantics but activates zero folds; it is explicitly not claimed as optimization coverage.

## Complete-call timings

The unchanged balanced eight-block cohort completes **3,968 primary /4,048 total calls** across all20 cases (largest33,027B). All original1.05 gates pass: **16 faster /4 slower medians**, geometric elapsed **-0.897836%**. Natural16KB and32KB win all eight paired blocks. Periodic and three tiny stored cases remain adverse; every contrary paired block is retained. Three zero reference medians stay unresolved. No compression sample is discarded or repeated.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.678007× | 3.619007× | -1.604148% | 8/0 |
| reymont-prefix-16384 | 4.419504× | 4.337101× | -1.864541% | 8/0 |
| random-32769 | 4.872262× | 4.843413× | -0.592118% | 4/4 |
| 16384-distribution-change | 6.437103× | 6.393901× | -0.671141% | 6/2 |
| repeated-32769 | 3.090523× | 3.051817× | -1.252410% | 5/3 |
| 33027-periodic-window | 3.518398× | 3.529740× | +0.322370% | 5/3 |
| eight-literals | 4.613850× | 4.565803× | -1.041376% | 6/2 |
| empty | unresolved | unresolved | +0.699232% | 1/7 |
| one-stored | unresolved | unresolved | +0.329487% | 3/5 |
| seven-stored-prefix | unresolved | unresolved | +0.382614% | 2/6 |
| eight-overlap | 4.417901× | 4.402003× | -0.359836% | 5/3 |
| 320-overlap | 4.342785× | 4.260022× | -1.905756% | 6/2 |
| eight-literals-prefix | 4.631750× | 4.579445× | -1.129262% | 6/2 |
| 128-biased-dynamic | 3.937016× | 3.913176× | -0.605524% | 4/4 |
| 32-high-entropy-stored | 4.246982× | 4.197657× | -1.161415% | 7/1 |
| random-64 | 4.275276× | 4.214598× | -1.419284% | 5/3 |
| random-256 | 4.348175× | 4.263199× | -1.954292% | 7/1 |
| random-1024 | 4.011117× | 3.936941× | -1.849265% | 7/1 |
| repeated-1024 | 4.156054× | 4.127331× | -0.691127% | 5/3 |
| reymont-prefix-1024 | 4.720228× | 4.648350× | -1.522758% | 7/1 |

Each pair shares the same fresh reference. Independent rounds are not compounded. All complete evaluate_into calls verify exact pinned bytes, independent inflate, equal logical work, unchanged input aliases, prefixes/capacities and managed cleanup zero. Logical work is not hardware instructions.

## Instructions, cycles and memory

Independent whole-call counters: **14 lower /6 higher instruction medians**, **5 higher cycle medians**. The five larger cases all execute more instructions while using fewer cycles. Eight bytes use fewer instructions but more cycles. This tradeoff is explicit; CPU-time samples are not used to partition retired instructions.

| Input | Instructions before | Instructions after | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 218,661,195.0 | 219,736,885.0 | +0.491944% | -2.882354% |
| random-32769 | 72,164,526.0 | 72,569,336.0 | +0.560954% | -0.391926% |
| 16384-distribution-change | 66,399,191.0 | 66,539,691.5 | +0.211600% | -1.659275% |
| repeated-32769 | 71,504,317.0 | 71,980,942.0 | +0.666568% | -5.046758% |
| 33027-periodic-window | 77,268,473.5 | 77,709,345.5 | +0.570572% | -2.042243% |
| eight-literals | 12,625,677.5 | 12,553,219.5 | -0.573894% | +1.819071% |

Allocation requests, requested bytes, reallocations and all measured live/incremental-peak heap metrics are exactly unchanged across20 cases. No heap/RSS/physical-memory saving is claimed. Full raw counter cohorts remain in the evidence.

All12 scalar controls pass original latency/allocation/no-slope gates; 5 adverse medians remain. Ratio range **0.996753054–1.005999246**.

## Current executable profiles

Five exact-current profiles complete **17,729 calls**, with every recorder exit, per-case cleanup and final managed bytes/nodes zero. An independent source-location build repeats the entire actual Object byte-for-byte. Unknown native source tags remain explicit; outside main includes useful host algorithms, generated helpers and harness validation. These are CPU-time samples, not an instruction ledger or removable-host fraction.

| Input | Running timer samples | Inside main | Native without fine tag |
|---|---:|---:|---:|
| eight-literals | 5526 | 9.591024% | 4.035469% |
| random-32769 | 5519 | 70.501903% | 34.843269% |
| 16384-distribution-change | 5364 | 50.894855% | 25.633855% |
| repeated-32769 | 5508 | 72.058824% | 28.594771% |
| reymont-prefix-32769 | 5517 | 80.913540% | 35.671561% |

## Validation and reproducibility

Four new maintained native tests cover all word widths, noncanonical integer branch conditions, unselected arms, joins, shared targets, fresh changed loop arguments, new loads after alias writes, error order, unchanged publication and retry. **660 native unit tests / all243 CI-selected names actually execute and pass**, with the same seven inherited exclusions. Unchanged **236 component /8 composition /1 real-capacity-path** tests pass. Fifteen architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings denied pass. No expectation or resource allowance increases. The actual16-row original/decoded source proof and resident comparison are retained, including the expected negative-parent coverage rejection.

Parent 2f7f38a4332c93cbaa6679a7138f7e0717fc9043; measured source 9e450f56e6d619e8bb8f5f3a9fef764afafd6435; baseline compiler source 2aa4229d7038dfa6dbf6aec73c60ca114386fa37. Object 8c6ea5085879904966bef5f5191059da15ab4ae75780aeb8c0eedf292db249ae; worker b9433e9fcfb11172ca789bdbf90d9131fd1e1a3dede1ead4e69ff36d168c1be0. Complete seven-file compiler/test/CI source, patch and original commit data are preserved. Final publication amends only docs/index; compiled source bytes remain equal.

The frozen source/MBC/algorithm/tables/depth300/nice258/EOF/ties/source allocation sites, real262144-entry U16 tree /1500513-word U32 cache /305257-entry U64 path capacity, output13clear/reuse, aliases, mimalloc2.1.7 and full evaluate_into boundary remain unchanged. FrozenR145 dependencies; no math-library rebuild. Existing draft#973 only; no new PR. Ordinary hosted CI is cancelled by instruction, review automation preserved and exact-head review requested without waiting. Broader lowering and1×parity remain incomplete.

Run checkpoint.py against chronological unpacked evidence to regenerate REPORT.md/results.json without compiling or compressing. Public receipt-only replay checks both output hashes and the packet replay record.
