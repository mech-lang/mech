# R190: identical private join arguments

Reject this exact join-alias variant: all20 original gates pass, but19 elapsed medians rise (geometric+0.783878%),14 instruction medians rise,13 cycle medians rise and all12 scalar medians are adverse. Text shrinks2456B but stack rises144B and heap stays equal. Preserve source and every observation; restore qualified R186. This rejects the measured tradeoff, not a small positive gain.

Production: 582c8b00cf86df926f21845573222c3cf9ba117f. Measured candidate source: 1429100a3672ab5303d8b4d68c13caee6f31e12c. Parent: b90561552b5f0cc0c894e26cc273746e2248d051.

All 20 original 1.05 gates: True. 1 faster / 19 slower medians; geometric elapsed +0.783878%. No sample discarded or repeated. All 3,968 timed / 4,048 total complete calls retain exact bytes, independent inflation, logical work, aliases, prefixes, capacities and zero final managed memory. Three zero reference medians remain unresolved.

| Input | Before / Roc | Candidate / Roc | Elapsed change | Improving/adverse blocks |
|---|---:|---:|---:|---:|
|reymont-prefix-32769|3.627365×|3.651736×|+0.671868%|1/7|
|reymont-prefix-16384|4.343284×|4.376853×|+0.772888%|3/5|
|random-32769|4.738701×|4.769069×|+0.640855%|3/5|
|16384-distribution-change|6.312078×|6.323377×|+0.179006%|3/5|
|repeated-32769|2.982935×|3.005232×|+0.747488%|1/7|
|33027-periodic-window|3.469895×|3.513236×|+1.249086%|3/5|
|eight-literals|4.480176×|4.512758×|+0.727233%|3/5|
|empty|unresolved|unresolved|-0.054213%|5/3|
|one-stored|unresolved|unresolved|+0.035012%|5/3|
|seven-stored-prefix|unresolved|unresolved|+0.017524%|5/3|
|eight-overlap|4.267148×|4.300241×|+0.775542%|2/6|
|320-overlap|4.191498×|4.221593×|+0.717984%|3/5|
|eight-literals-prefix|4.522801×|4.562792×|+0.884214%|2/6|
|128-biased-dynamic|3.807146×|3.897373×|+2.369941%|2/6|
|32-high-entropy-stored|4.140570×|4.176463×|+0.866861%|3/5|
|random-64|4.108575×|4.151167×|+1.036665%|2/6|
|random-256|4.254353×|4.319421×|+1.529457%|1/7|
|random-1024|3.760918×|3.801849×|+1.088321%|1/7|
|repeated-1024|4.125581×|4.140334×|+0.357601%|3/5|
|reymont-prefix-1024|4.637651×|4.688382×|+1.093897%|1/7|

## Separate hardware and heap cohorts

14 higher instruction medians; 13 higher cycle medians. All 20 observed heap metrics equal: True. Timer samples, retired instructions, cycles, requested bytes, copied bytes and live memory are separate quantities. No per-function instruction count is inferred from CPU-time shares.

| Input | Instructions before | Candidate | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|reymont-prefix-32769|215874863.0|217383750.0|+0.698964%|+0.046899%|
|reymont-prefix-16384|122795341.5|123425783.0|+0.513408%|+0.403686%|
|random-32769|71372597.0|71270405.0|-0.143181%|+0.808395%|
|16384-distribution-change|65606164.0|65586424.5|-0.030088%|-0.063431%|
|repeated-32769|71097191.0|71135961.5|+0.054532%|+0.135522%|
|33027-periodic-window|76779398.5|76813300.0|+0.044154%|+0.751285%|
|eight-literals|12449986.0|12450216.0|+0.001847%|+1.301637%|
|empty|1060443.0|1062027.5|+0.149419%|+0.749806%|
|one-stored|1112862.0|1114423.5|+0.140314%|+1.266438%|
|seven-stored-prefix|1112726.0|1114195.0|+0.132018%|+0.980443%|
|eight-overlap|11647307.0|11647545.5|+0.002048%|-1.007265%|
|320-overlap|11783427.0|11784424.5|+0.008465%|+0.864203%|
|eight-literals-prefix|12450237.0|12452863.0|+0.021092%|-0.669972%|
|128-biased-dynamic|10415737.5|10417093.0|+0.013014%|+5.096839%|
|32-high-entropy-stored|11028836.0|11028813.0|-0.000209%|-0.970470%|
|random-64|11184983.5|11184894.5|-0.000796%|-1.045176%|
|random-256|11710573.0|11709228.5|-0.011481%|-0.896854%|
|random-1024|11923158.0|11915914.5|-0.060752%|-0.877868%|
|repeated-1024|13425684.0|13427172.5|+0.011087%|+0.412253%|
|reymont-prefix-1024|16183910.0|16190010.5|+0.037695%|+0.249073%|

## Mechanism and verification

One conservative sweep aliases 142 private CFG formals receiving an identical current SSA vector on every incoming edge. It compares exact SSA identity, not equal-looking arithmetic. Types, duplicate-target edges, edge-only arguments, source effects and every current invocation remain checked. The existing shared replacement routine rewrites private signatures. No source operation, charge, permission, cache, runtime ABI or allocation site changes. IR210/work107/ABI39/MBC14.

Accounted generated text: 8,153,276 → 8,150,820 bytes (−2,456); reported entry stack: 16,480 → 16,624 bytes (+144). The register-allocation change is a measured tradeoff, not a proved spill reduction. Root source and bytecode hashes stay fixed.

The parent optimization witness fails as intended; seven focused tests pass with the candidate. Controls cover changing nested visits, six integer/float representations including NaN bits, different selected vectors, observable stores, early rejection, unchanged prior publication and retry. The 664-test native suite retains the same seven inherited exclusions; all 247 scheduled CI names execute and pass. Warnings denied, 15 architecture/catalog checks, 132 workflow tests, runtime-only configuration and formatting pass. Unchanged component/composition/real-capacity qualification is 236/8/1.

All 12 original/decoded scalar controls complete; original gate checks pass: True. Five fresh exact-candidate profiles complete 17,748 correct calls with zero final managed bytes/nodes. Their source-map compiler independently repeats the whole actual Object byte-for-byte. Outside-main samples include other generated helpers, useful host algorithm work and the harness; they are not all removable overhead.

| Input | Main generated entry | Outside main entry |
|---|---:|---:|
|eight-literals|9.3811%|90.6189%|
|random-32769|71.3381%|28.6619%|
|16384-distribution-change|52.1350%|47.8650%|
|repeated-32769|71.5112%|28.4888%|
|reymont-prefix-32769|79.6048%|20.3952%|

The actual native prologues keep the same saved-register set, while fixed stack storage grows 16,368 → 16,512 bytes. The additional 144 bytes belong to register-allocation storage; no new explicit stack array is introduced. Fresh candidate unattributed stack accesses account for21.16% of natural32,25.31% random32,21.31% repeated32 and16.45% distribution-change16 timer samples. These are separate profile observations, not a paired spill-rate or retired-instruction claim.

The separate heap-instrumented OS cohort has lower resident medians (about229–311KB on the selected examples) and163,776 fewer physical-footprint bytes. Heap owners, requested bytes, reallocations and incremental usable peaks remain exactly equal. The OS reduction does not establish fewer input copies, streaming or a primary elapsed gain.


Compiler: nightly-2026-03-03/aarch64; GPU library opt-level3, unit/component roots opt-level1; warnings denied. Native test features: aot, canonical-native, canonical-native-program, canonical-native-program-runtime, canonical-native-runtime, jit. Exact commands and environment are retained in build/test/check receipts; validation-provenance.json reconciles the pre-commit builds with all eight measured Git blobs.


Fresh Clang and pinned Rust join diagnostics are separately retained and never linked into compression. The retained reference excerpt is historical pinned inspection, while every primary timing uses a fresh pinned reference run.

Skipped: the same seven inherited native exclusions, fresh dependency/Core/Engine/math/full-feature/minimal rebuilds, standalone linked harness, multi-megabyte runs and the separate ten-workload suite. Runtime/module builds reuse frozen R145 dependencies. Hosted ordinary CI is cancelled by instruction; no green hosted CI or clean exact-head external review is claimed. PR973 stays draft.

Introduced setup failures and their repairs are in setup-failures.json and original logs: a non-orderable type key, a not-yet-created working directory, and premature help/counter preparation calls. None started or altered a timing cohort. The unchanged source/resource/output controls pass; no unresolved introduced functional failure is claimed.

Evidence reconstructs from chronological immutable packets with complete candidate source/patch/raw commit, fixture pins and receipts. No native binary, bytecode, raw trace store, credential or private environment export is published. Parity remains incomplete.

[Verified evidence container](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r190-evidence-container-b898c74cfc6a.tar.gz) · [All 38 hash receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r190-archive-receipts.json). The container preserves all37 chronological packets; the measured source is reconstructed from its parent and the archived complete patch, rather than retained on the branch.

Fresh public replay verifies all37 inner packets,4,384 file occurrences/1,007 paths,13 multiversion paths,all38 receipts and three byte-identical outputs. No compiler or compressor is run by numerical replay.
