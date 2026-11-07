# Native unsigned interval predicates — R184

Retain the exact unsigned predicate simplification and code/stack reduction with all original gates intact. Primary elapsed is a modest mixed observation, not a broad instruction-count win. No statistical significance or parity claim.

The compiler combines unsigned constant bounds of one immutable SSA word. For 0≤L≤U<2^w, L≤x≤U is equivalent to ((x−L) mod2^w)≤U−L. Signed/dynamic bounds, different SSA words, empty/overflowing strict intervals and unsupported casts retain the original lowering. Source checks, effects, error order, work debits and publication remain. This is private predicate arithmetic, not a change to checked source arithmetic.

Four real compiled sites replace two compares and Boolean conjunction with one wrapping interval test. All earlier numeric recipes/windows/proof selections remain byte-for-byte equal. Generated text 8,468,144 → 8,461,948 B (−6,196); stack 17,008 → 16,976 B (−32). IR/work/ABI/MBC 204/107/39/14. No source, allocator, ABI or extra backend-pass change.

## Complete-call timing

All 20 original 1.05 gates pass; 14 faster / 6 slower medians, geometric elapsed -0.191492%. The unchanged balanced eight-block cohort contains3,968 primary / 4,048 total calls, largest 33,027 B. Every adverse block remains. Three zero reference medians stay unresolved. No compression sample is discarded or repeated. Startup-only failures are documented below. Ratios share the same fresh reference within each pair; independent rounds are not compounded.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.678092× | 3.670159× | -0.215685% | 5/3 |
| reymont-prefix-16384 | 4.457854× | 4.433658× | -0.542781% | 6/2 |
| random-32769 | 4.840132× | 4.804452× | -0.737176% | 6/2 |
| 16384-distribution-change | 6.391936× | 6.391698× | -0.003708% | 4/4 |
| repeated-32769 | 3.016813× | 2.999579× | -0.571256% | 4/4 |
| 33027-periodic-window | 3.555805× | 3.509126× | -1.312767% | 6/2 |
| eight-literals | 4.577226× | 4.522629× | -1.192787% | 6/2 |
| empty | unresolved | unresolved | -0.089796% | 5/3 |
| one-stored | unresolved | unresolved | -0.173209% | 5/3 |
| seven-stored-prefix | unresolved | unresolved | -0.052002% | 5/3 |
| eight-overlap | 4.407550× | 4.388384× | -0.434860% | 4/4 |
| 320-overlap | 4.276573× | 4.267291× | -0.217050% | 4/4 |
| eight-literals-prefix | 4.566082× | 4.603947× | +0.829265% | 3/5 |
| 128-biased-dynamic | 3.754196× | 3.797568× | +1.155294% | 3/5 |
| 32-high-entropy-stored | 4.122836× | 4.162917× | +0.972178% | 4/4 |
| random-64 | 4.255392× | 4.277388× | +0.516909% | 3/5 |
| random-256 | 4.234323× | 4.236134× | +0.042770% | 4/4 |
| random-1024 | 3.928650× | 3.932094× | +0.087680% | 3/5 |
| repeated-1024 | 4.159744× | 4.150557× | -0.220839% | 4/4 |
| reymont-prefix-1024 | 4.771146× | 4.693818× | -1.620729% | 5/3 |

Every full evaluate_into call verifies exact pinned bytes, independent inflate, equal work, unchanged input aliases, prefixes/capacities and final managed cleanup zero. Logical work is not hardware instructions.

## Instructions and memory

Independent hardware medians: 6 lower / 14 higher instruction counts; 10 higher cycle counts. These contrary observations are retained. No function-level retired-instruction allocation is inferred from CPU-time samples.

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 220,699,323.0 | 218,676,812.0 | -0.916410% | +0.398174% |
| random-32769 | 72,367,859.5 | 72,162,967.0 | -0.283126% | +0.911266% |
| 16384-distribution-change | 66,507,471.0 | 66,399,306.0 | -0.162636% | -0.067630% |
| repeated-32769 | 71,505,112.0 | 71,501,937.5 | -0.004440% | +4.157906% |
| 33027-periodic-window | 77,236,490.5 | 77,270,808.0 | +0.044432% | +4.329399% |
| eight-literals | 12,544,161.5 | 12,624,466.5 | +0.640178% | -0.063287% |

Allocation requests, requested bytes, reallocations and all measured live/incremental-peak heap metrics are exactly unchanged across 20 cases. Smaller code/stack is not claimed as a measured heap/RSS saving. Full raw hardware/RSS/physical/heap cohorts remain in results.json and their receipts.

Twelve scalar controls pass the original 1.05 latency and allocation gates;4 adverse medians remain, ratios0.994125459–1.001977965.

## Current executable profile

Five exact-current profiles contain 17,656 complete correct calls; all recorder exits, per-case cleanup and final managed bytes/nodes are zero. Independent source-location compilation repeats the whole actual Object byte-for-byte. Source labels can be lost or shared by backend folding/hoisting, so unknown native samples remain explicit. Outside main includes useful runtime algorithms, generated helpers and harness validation. These are CPU-time samples, not instruction counts or a removable-host fraction.

| Input | Running timer samples | Inside main | Native without fine tag |
|---|---:|---:|---:|
| eight-literals | 5549 | 9.425122% | 3.910615% |
| random-32769 | 5364 | 70.805369% | 33.370619% |
| 16384-distribution-change | 5354 | 51.382144% | 25.532312% |
| repeated-32769 | 5508 | 72.639797% | 28.885258% |
| reymont-prefix-32769 | 5529 | 80.249593% | 34.924941% |

## Correctness and provenance

Two new native tests cover 8/16/32/64-bit boundaries, high unsigned values, strict/reversed/immediate/widened forms, retained effects and refusal of unsafe patterns. The maintained source test covers original/decoded interval assertion rejection before failing subtraction, prior publication, cleanup and recovery. The complete 656 GPU tests / all 239 CI-selected names actually execute and pass, with the same seven inherited exclusions. Unchanged 236 component / eight composition / one real-capacity-path tests pass. Fifteen architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings denied pass. No expected value or allowance increase.

Harness setup repairs are preserved: stale identity symbol at link, then copied old descriptor rejected by image admission. Both are corrected before any compression call; zero measured samples were discarded. The initial unstaged patch omitted the new module; its hash was already recorded and the complete seven-file commit, retained source and full patch preserve all bytes. Copied GPU recipe labels also retained an older revision/count (654); qualification-label-erratum.json binds the actual current binary/source manifest and 656 executed tests without rewriting history. One early inventory setup parsed only the second continued selector line; the corrected maintained selector lists239tests and all239execute in the final suite.

Parent 3471b729c880c24cce9d05b3821ffe6b4633adfc; measured compiler source 2aa4229d7038dfa6dbf6aec73c60ca114386fa37; baseline source 8fc62da7db473ba744b804af7a16cdb8d4dfb2ec. Object 72055c76562176d6e3be34e550e2af47a8062f63b48a9721236d462e9abfd74f; worker 646f5b6d2ab15879bc96ca55ae1d2d8d6f4d3800ba11d137634b6f9dc0bc86ee. The final publication amendment changes documentation/index only; exact compiler/test/CI file hashes remain pinned in complete-candidate-source-files.json.

Pinned source/MBC/algorithm/tables/search depth 300 / nice length 258 / EOF/ties/source allocation sites, real 262144-entry U16 tree / 1500513-word U32 cache / 305257-entry U64 path capacity, output capacity 13 with clear/reuse, aliases, mimalloc 2.1.7 and full evaluate_into remain unchanged. Frozen R145 dependencies; no math-library rebuild. Existing draft #973 only; no new PR. Ordinary hosted CI is cancelled by instruction, exact-head review is requested without waiting. Broader lowering and 1× parity remain incomplete.

Run checkpoint.py against the unpacked chronological evidence to regenerate this report and results.json without compiling or compressing. Both output hashes are checked by publication replay.
