# Current compression scoreboard

**45/81 cases faster than Roc (55.56%).** Goal 100%.
R279 separates packet descriptor lifetimes across the checked append run. Paired elapsed changes -0.509% across all 81. Both timing gates pass 81/81. The separate libdeflate cohort has 3 faster cases.

| Input | R278 / Roc | R279 / Roc | R278 / libdeflate | R279 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.201945× | 0.201739× | 2.532281× | 2.529701× |
| prefix-random-8192 | 0.968319× | 0.973673× | 3.468744× | 3.487925× |
| prefix-periodic-131072 | 0.953066× | 0.946454× | 1.720157× | 1.708223× |
| prefix-periodic-1048576 | 0.975436× | 0.969030× | 1.649626× | 1.638792× |
| prefix-repeated-131072 | 0.933553× | 0.929834× | 1.566924× | 1.560683× |
| prefix-repeated-1048576 | 0.954446× | 0.948688× | 1.501559× | 1.492502× |
| prefix-natural-4096 | 1.103297× | 1.093769× | 1.886769× | 1.870476× |
| prefix-natural-1048576 | 1.527874× | 1.522098× | 1.232274× | 1.227616× |
| prefix-random-1048576 | 1.976420× | 1.975365× | 2.793264× | 2.791773× |

Ratios use the same fresh four-actor comparison. The parity score and gates use the two primary cohorts.

[R279 timing, proof and replay evidence](native-append-descriptor-lifetimes.md).

## Historical R278 measured comparison

**45/81 cases faster than Roc (55.56%).** Goal 100%.
R278 specializes the checked empty append run. Paired elapsed changes -4.382% across all 81. Both timing gates pass 81/81. The separate libdeflate cohort has 3 faster cases.

| Input | R276 / Roc | R278 / Roc | R276 / libdeflate | R278 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.198646× | 0.198668× | 2.523829× | 2.524102× |
| prefix-random-8192 | 1.040506× | 0.971396× | 3.726015× | 3.478534× |
| prefix-periodic-131072 | 1.032247× | 0.947082× | 1.878022× | 1.723077× |
| prefix-periodic-1048576 | 1.062911× | 0.976064× | 1.798539× | 1.651587× |
| prefix-repeated-131072 | 1.012677× | 0.938080× | 1.700352× | 1.575098× |
| prefix-repeated-1048576 | 1.034364× | 0.954039× | 1.625450× | 1.499224× |
| prefix-natural-4096 | 1.083503× | 1.099890× | 1.865332× | 1.893543× |
| prefix-natural-1048576 | 1.501933× | 1.527530× | 1.210507× | 1.231137× |
| prefix-random-1048576 | 2.124785× | 1.983389× | 3.000450× | 2.800781× |

Ratios use the same fresh four-actor comparison. The parity score and gates use the two primary cohorts.

[R278 timing, proof and replay evidence](native-checked-append-run.md).

## Historical R276 measured comparison

**38/81 cases faster than Roc (46.91%).** Goal 100%.
R276 evaluates the first short ordered-reduction visit directly. Paired elapsed changes -0.289% across all 81. Both timing gates pass 81/81. The separate libdeflate cohort has 3 faster cases.

| Input | R275 / Roc | R276 / Roc | R275 / libdeflate | R276 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.197751× | 0.198232× | 2.520126× | 2.526258× |
| prefix-repeated-131072 | 1.012775× | 1.012343× | 1.699752× | 1.699029× |
| prefix-repeated-1048576 | 1.034131× | 1.035177× | 1.626138× | 1.627783× |
| prefix-periodic-1048576 | 1.062649× | 1.066074× | 1.801070× | 1.806875× |
| prefix-natural-32768 | 1.478091× | 1.471887× | 1.341623× | 1.335991× |
| prefix-natural-1048576 | 1.508769× | 1.499167× | 1.219665× | 1.211903× |
| prefix-random-1048576 | 2.117142× | 2.124698× | 2.999997× | 3.010703× |

Ratios use the same fresh four-actor comparison. The parity score and gates use the two primary cohorts.

[R276 timing, proof and replay evidence](native-short-reduction-first-visit.md).

R277 measures the dense offset map as a source experiment. Paired elapsed changes -0.320% across 81 cases. Both timing gates pass 81/81. The compiler and source baseline stays at R276. [R277 results and replay](native-dense-offset-slot-experiment.md).

## Historical R275 measured comparison

**38/81 cases faster than Roc (46.91%).** Goal 100%.
R275 reuses proved byte equality across shifted immutable input windows and keeps the R272 runtime. Paired elapsed falls 2.55% across all 81. Both timing gates pass 81/81. 6 cases move below Roc parity. The separate libdeflate cohort has 3 faster cases.

| Input | R274 / Roc | R275 / Roc | R274 / libdeflate | R275 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.199350× | 0.200462× | 2.536363× | 2.550512× |
| prefix-periodic-32768 | 0.985956× | 0.908698× | 2.109599× | 1.944295× |
| prefix-periodic-65536 | 1.076877× | 0.991375× | 2.070482× | 1.906090× |
| prefix-repeated-65536 | 1.044666× | 0.972245× | 1.843924× | 1.716094× |
| prefix-repeated-131072 | 1.095238× | 1.015547× | 1.831637× | 1.698365× |
| prefix-repeated-262144 | 1.115580× | 1.032471× | 1.809245× | 1.674460× |
| prefix-periodic-1048576 | 1.159278× | 1.063635× | 1.963402× | 1.801417× |
| prefix-repeated-1048576 | 1.121432× | 1.035256× | 1.761369× | 1.626018× |
| prefix-natural-1048576 | 1.508668× | 1.512215× | 1.215593× | 1.218451× |
| prefix-random-1048576 | 2.111450× | 2.121498× | 2.982260× | 2.996453× |

The table uses a fresh four-actor cohort. The parity score and timing gates use the two primary cohorts.

[R275 compiler patch, all 81 timings, compiled controls and profiles](native-shifted-immutable-equality.md).

## Historical R274 measured comparison

**32/81 cases faster than Roc (39.51%).** Goal 100%.
R274 carries the selected item in capacity-proved native reductions and keeps the R272 runtime. Paired elapsed falls 3.09% across all 81. Both timing gates pass 81/81. The matched control and candidate each beat Roc in 32 cases. The separate libdeflate cohort has three faster cases for each.

| Input | R272 / Roc | R274 / Roc | R272 / libdeflate | R274 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.201229× | 0.199891× | 2.554727× | 2.537733× |
| prefix-repeated-33027 | 0.981701× | 0.971683× | 1.904939× | 1.885501× |
| prefix-periodic-32768 | 0.982359× | 0.980489× | 2.096360× | 2.092368× |
| prefix-repeated-65536 | 1.050521× | 1.041300× | 1.858888× | 1.842571× |
| prefix-natural-32768 | 1.685446× | 1.475704× | 1.535526× | 1.344440× |
| prefix-natural-1048576 | 1.760098× | 1.506684× | 1.419691× | 1.215288× |
| prefix-random-1048576 | 2.108295× | 2.107406× | 2.974567× | 2.973314× |
| prefix-periodic-1048576 | 1.157258× | 1.150178× | 1.971536× | 1.959475× |
| prefix-repeated-1048576 | 1.120134× | 1.120273× | 1.762614× | 1.762834× |

The table uses a fresh four-actor cohort. The parity score and timing gates use the two primary cohorts.

[R274 compiler patch, all 81 timings, linked controls and profiles](native-capacity-phase-selected-item.md).

## Historical R272 measured comparison

**31/81 cases faster than Roc (38.27%).** Goal 100%.
R272 supplies large zero-filled buffers through private pages. The compressor Object and program stay at R270. Paired elapsed falls 10.25% across all 81. Both timing gates pass 81/81. Four periodic inputs cross below Roc parity. The separate libdeflate cohort has 3/81 faster cases.

| Input | R270 / Roc | R272 / Roc | R270 / libdeflate | R272 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.416211× | 0.197800× | 5.324992× | 2.530654× |
| prefix-repeated-4096 | 0.673301× | 0.528675× | 2.543811× | 1.997397× |
| prefix-repeated-33027 | 1.001379× | 0.982016× | 1.946909× | 1.909263× |
| prefix-periodic-32768 | 1.012630× | 0.988405× | 2.170907× | 2.118972× |
| prefix-natural-1048576 | 1.765329× | 1.756943× | 1.425009× | 1.418240× |
| prefix-random-1048576 | 2.117091× | 2.106640× | 2.992581× | 2.977810× |

Ratios in this table use the same fresh four-actor cohort. The 31-case score comes from the two primary cohorts.

[R272 full timings, memory policy, qualification and profiles](native-zero-page-scratch.md).

R273 reduces work in read-only scalar loops. Empty and one-byte inputs miss the block timing gate, so R272 remains active. [R273 measurements and exact-image evidence](native-readonly-scalar-reductions.md).

## Historical R270 measured comparison

**27/81 cases faster than Roc (33.3%).** Goal 100%.
R270 is retained. Paired elapsed improves 0.737% over R268, with 81/81 passes on both gates. Repeated 33,027 bytes crosses parity. All previously faster cases stay below 1× in the paired comparison.

| Frontier | R268 / Roc | R270 / Roc |
|---|---:|---:|
| prefix-repeated-32768 | 0.995036× | 0.973240× |
| prefix-repeated-33027 | 1.015880× | 0.992047× |
| prefix-periodic-32768 | 1.033342× | 1.002207× |
| prefix-periodic-32767 | 1.030812× | 1.008713× |
| eight-literals | 0.426039× | 0.426594× |
| prefix-natural-1048576 | 1.750283× | 1.764205× |

[R270 timings, qualification and compiler patch](native-equality-loop-tiers.md). [Mech against libdeflate and Roc](native-libdeflate-comparison.md).

R271 removes the recurring previous-node load and changes paired elapsed by -0.149%. The timing gates miss seven-stored-prefix. R270 remains retained. [R271 measurements and proof](native-append-tail-forwarding.md).

## Historical R268 measured comparison

**Cases faster than Roc: 26/81 = 32.1%. Goal 100%.**
R268 is the retained compact compiler. The growth-isolated threaded candidate changes paired elapsed -3.590% across all 81. It passes 81/81 pooled and 81/81 block gates.

| Frontier | R264 / Roc | R268 candidate / Roc |
|---|---:|---:|
| prefix-repeated-32767 | 1.01982× | 0.99346× |
| prefix-repeated-32768 | 1.02010× | 0.98651× |
| prefix-repeated-32769 | 1.02155× | 0.99224× |
| prefix-repeated-33027 | 1.03488× | 1.01076× |
| prefix-periodic-32768 | 1.04986× | 1.02095× |
| eight-literals | 0.42972× | 0.42657× |
| prefix-natural-1048576 | 1.88820× | 1.75372× |

Cases moving below 1× in the fresh comparison repeated-32769, prefix-repeated-32767, prefix-repeated-32768, prefix-repeated-32769.

[All 81 results, proof, counters, safety gates and profiles](native-thread-after-growth-isolation.md).

The R269 vector address pilot missed both eight-literal timing gates and moved three cases above parity in its screen. R268 remains the measured baseline. [R269 screen and executable evidence](native-shared-vector-addresses.md).

## Historical R267 measured comparison

**Cases faster than Roc: 22/81 = 27.2%. Goal 100%.**
R264 remains the retained experimental compiler. The separate no-growth body improved paired elapsed 2.1243% across the matrix. Empty and one-byte inputs missed the block timing gate. Both counter cohorts and all adverse observations are preserved.

| Frontier | R264 / Roc | No-growth candidate / Roc |
|---|---:|---:|
| prefix-repeated-32767 | 1.02345× | 1.00157× |
| prefix-repeated-32768 | 1.02433× | 1.00468× |
| prefix-repeated-32769 | 1.03258× | 1.00284× |
| prefix-periodic-32768 | 1.05766× | 1.03260× |
| eight-literals | 0.41593× | 0.41354× |
| prefix-natural-1048576 | 1.89164× | 1.81871× |

In the separate 13-case screen, applying R260 after growth-state isolation moves repeated 32,768 bytes from 1.0055× to 0.9945× Roc. Repeated 32,767 bytes reaches 0.9951×. Both gates pass for all 13 cases. The full 81-case acceptance follows separately.

[Full R267 matrix, allocator trace, capacity proof and pilots](native-loop-state.md).

## Historical R266 measured rejection

**Cases faster than Roc: 22/81 = 27.2%; goal 100%.**
Retained experimental compiler: **R264**. R266 is rejected globally: direct
extending loads reduce instructions but slow the near-parity frontier. Its
qualified instruction savings and eight-byte improvement remain available for
later isolated or combined measurements.

| Headline | Retained R264 | Evaluated R266 | Movement |
|---|---:|---:|---:|
| Faster-than-reference configurations | 22/81 (27.1605%) | 22/81 (27.1605%) | 0.0000 percentage points |
| Target | 100% | 100% | Open |
| All 81 paired geometric elapsed | Baseline | +1.747846% | Candidate rejected |
| Primary cohorts | Baseline | +1.702110%, +1.793602% | Both adverse |
| Existing pooled/block 1.05 gates | — | 81/81 each | Unchanged ceilings |
| Heap/copy/zero/private peaks | Baseline | Equal on all 81 | No saving |

A case counts only when its paired geometric elapsed ratio is strictly below 1.0
in the same fresh balanced blocks; ties do not count. The fixed 81-case inventory
is unchanged. This is a configuration share, not byte-weighted throughput or a
confidence threshold. Chat updates should show this metric and compact frontier;
all 81 cases continue to be measured and saved in the round report.

| Frontier / control | Bytes | Retained R264 / Roc | Rejected R266 / Roc | Time change |
|---|---:|---:|---:|---:|
| prefix-repeated-32767 | 32767 | 1.018886x | 1.044322x | +2.4965% |
| prefix-repeated-32768 | 32768 | 1.023706x | 1.049353x | +2.5053% |
| prefix-repeated-32769 | 32769 | 1.028317x | 1.058131x | +2.8993% |
| prefix-repeated-33027 | 33027 | 1.041527x | 1.067907x | +2.5329% |
| prefix-periodic-32768 | 32768 | 1.057612x | 1.086396x | +2.7215% |
| prefix-periodic-32769 | 32769 | 1.066628x | 1.094666x | +2.6287% |
| eight-literals | 8 | 0.431513x | 0.422142x | -2.1717% |

The candidate cuts repeated/periodic 32 KiB retired instructions about 1.88% but
slows their primary elapsed about 2.5–2.7%. Eight literals improves 2.17% elapsed
and 1.79% instructions. Natural 1 MiB instructions decline 3.00% while primary elapsed
worsens 0.84%. These separate cohorts and positive/negative observations remain.
Both columns use the same fresh reference measurements; the retained compiler
is unchanged and historical denominators are preserved. No new case crosses 1x.

[All 81 rows, shared-load design, counters, C/Rust/reference assembly, source controls and review contract](native-extending-scalar-loads.md).
The path has six adjacent narrow-load/extension-or-copy pairs before and zero
after, but that is a static count. The selected matching frame shrinks 960→944B;
normal matching code grows 796B. The root frame shrinks 64B. The 2,084,296-byte
worker file size remains unchanged, and no heap/RSS saving is claimed.

725 native tests pass with seven inherited exclusions, plus five focused tests;
7384 native source controls, 759 fresh resident calls and 324 original/decoded full
smokes pass. All 17 image pairs agree. Eight new candidate profiles follow all
four measurement cohorts. R264 baseline/reference profiles and R230 matching
resident rows remain inherited. Full hosted CI, a fresh full linked/AOT suite,
full 6.6 MB input and the additional ordinary managed-contract integration remain
outside the current claims. The maintained 680-test result is inherited.

Next simplify the generic unit-append spare-capacity check: for maximum 1,
`length <= capacity && capacity-length >= 1` is exactly unsigned `length < capacity`.
The actual retained path currently builds that test with eight instructions.
Keep other maxima and the same cold growth/rejection path; qualify the unsigned
extremes, source errors/aliases/recovery and all 81 performance before adoption.
This proposal is not yet implemented. The target remains 100% faster cases.

Review requests must name exact base/head, linked source/evidence, metric
expectations, safety invariants and coverage limits.

## Historical R265 measured rejection

**Cases faster than Roc: 22/81 = 27.2%; goal 100%.**
Retained experimental compiler: **R264**. R265's checked internal argument-read
alternative is rejected: paired geometric elapsed **+1.812837%**, with both
primary cohorts adverse (+2.055305% and +1.570945%). Its isolated natural-text
gain is preserved in the report. No slower candidate is adopted.

| Headline metric | Retained R264 | Rejected R265 | Movement |
|---|---:|---:|---:|
| Cases faster than reference | 22/81 (27.1605%) | 22/81 (27.1605%) | 0.0000 percentage points |
| Target | 100% | 100% | Open |
| Existing pooled/block 1.05 gates | — | 81/81 each | Unchanged ceilings |
| Heap/copy/zero/private peaks | Baseline | Equal on all 81 | No saving |

Count a case only when its paired geometric elapsed ratio is strictly below 1.0
in the same fresh balanced blocks. Ties do not count. The denominator remains
the fixed 81 complete compression inputs/configurations; this is a case share,
not byte-weighted throughput or a significance threshold. Future chat updates
should show this percentage/count, percentage-point movement, and a compact
frontier. The complete matrix is still measured and saved in the round report.

| Frontier / control | Bytes | Retained R264 / Roc | Rejected R265 / Roc | Time change |
|---|---:|---:|---:|---:|
| prefix-repeated-32767 | 32767 | 1.026606x | 1.060901x | +3.3406% |
| prefix-repeated-32768 | 32768 | 1.031579x | 1.066543x | +3.3893% |
| prefix-repeated-32769 | 32769 | 1.030045x | 1.059532x | +2.8627% |
| prefix-repeated-33027 | 33027 | 1.044947x | 1.072964x | +2.6812% |
| prefix-periodic-32768 | 32768 | 1.059505x | 1.098785x | +3.7074% |
| prefix-periodic-32769 | 32769 | 1.071074x | 1.102238x | +2.9097% |
| eight-literals | 8 | 0.422626x | 0.421959x | -0.1578% |

Both columns use the same new reference measurements. R264's source is unchanged;
its historical ratios below keep their original denominators. There is no new
parity crossing. The candidate removes static reads but adds about 0.23% retired
instructions on repeated/periodic 32 KiB. This negative result is preserved.

[All 81 rows, percentage metric, counters, source proof, safety controls and review contract](native-current-packet-reads.md).
724 candidate native tests pass (seven inherited exclusions), 7384 fresh native
source calls, 759 fresh resident calls and 324 complete original/decoded smokes
pass. Eight new candidate profiles follow completion of all four cohorts. R264
baseline/reference profiles and R230 matching-resident rows are explicitly reused.
The unchanged maintained 680-test result is inherited; full hosted CI, a fresh full
linked/AOT suite and full 6.6 MB input are not claimed. The compact entry's ordinary
managed-contract omissions remain explicit. No source algorithm, buffer, table,
allocation site or timing boundary changes.

Next map the changed path-loop live ranges/spill sequence and dynamic visits
before another argument-read experiment; preserve live-value allocation and
judge the frontier and all 81 elapsed/instruction results. Review requests must
name exact base/head, linked patch/evidence, metric expectations, safety invariants
and coverage limits. Goal: 100% of cases faster than the reference.

## Historical R264 matched cohort

**Retained experimental computational baseline: R264 on R263.**
The compiler branches directly on vector truth while retaining normalized Boolean
values for observers. The source algorithm and timing boundary stay unchanged.

| Paired result, all 81 | R263 to R264 |
|---|---:|
| Geometric elapsed | **−0.336093%** |
| Cohort A | +0.094530% |
| Cohort B | −0.764863% |
| Existing pooled/block gates | 81/81 each |
| Below-reference configurations | 22 to 22 |
| Heap/copy/zero/private-peak families | Equal on all 81 |

This is a small, noisy gain with explicit tradeoffs. The next priority remains
the repeated/periodic cases nearest 1x around 32 KiB. No case newly crosses 1x.
Both columns below use the same fresh reference runs; historical tables retain
their original denominators. Negative changes mean faster.

| Case | Bytes | R263 / Roc | R264 / Roc | Time change |
|---|---:|---:|---:|---:|
| prefix-repeated-32767 | 32767 | 1.021118x | 1.022805x | +0.1652% |
| prefix-repeated-32768 | 32768 | 1.041336x | 1.034897x | -0.6183% |
| prefix-repeated-32769 | 32769 | 1.026890x | 1.026793x | -0.0094% |
| prefix-repeated-33027 | 33027 | 1.038928x | 1.040467x | +0.1481% |
| prefix-periodic-32767 | 32767 | 1.061943x | 1.066311x | +0.4113% |
| prefix-periodic-32768 | 32768 | 1.061908x | 1.057479x | -0.4170% |
| prefix-periodic-32769 | 32769 | 1.063255x | 1.061318x | -0.1822% |
| prefix-periodic-33027 | 33027 | 1.071861x | 1.068822x | -0.2835% |
| eight-literals | 8 | 0.428201x | 0.424475x | -0.8701% |
| prefix-natural-1048576 | 1048576 | 1.889632x | 1.890300x | +0.0353% |
| prefix-random-1048576 | 1048576 | 2.145590x | 2.149057x | +0.1616% |
| prefix-periodic-1048576 | 1048576 | 1.220897x | 1.222641x | +0.1428% |
| prefix-repeated-1048576 | 1048576 | 1.180242x | 1.180345x | +0.0087% |

[All81 comparisons, compiler/backend design, counters, profiles and evidence](native-vector-truth-lowering.md).
The actual private difference test shrinks 4→3 instructions and needs one scalar
lane transfer instead of two. Both matching frames grow 16 bytes; their normal
body shrinks 1,632 bytes and vector body grows 120 bytes. Root Object bytes, 339 of 341
root/numeric definitions and the 2,084,296-byte worker file size remain exact.
Natural/random 1 MiB instructions increase 0.27%/0.75%, while repeated/periodic 32 KiB
instructions fall about 0.35%. These costs remain beside the elapsed results.

720 native tests, 5788 fresh native source controls and 324 final complete
original/decoded smokes pass. Frozen source-resident observations are reused;
zero new source-resident calls are claimed. Seven native exclusions and the
unchanged maintained 680-test result remain inherited. Full hosted CI, a fresh full
linked/AOT suite, full 6.6 MB input and a fresh full disabled-pipeline emit are not
claimed. The established compact-entry ordinary managed-contract omissions stay
explicit. All primary/counter collection finishes before post-change profiling.
Twelve fresh exact-image captures include both 32 KiB border inputs.

Next retest the archived R235 checked argument-packet read alternative on the
current outlined architecture. Its old repetitive regression remains evidence;
no new win is assumed. Prove the current private packet span, retain source-array
traps/current arguments/aliases/error/retry behavior, and time all 81 again.

## Historical R263 matched cohort

**Historical R263 checkpoint, on R261's correctness repairs.**
This generic compiler change reuses guarded current-length bounds for masked
one-based indices. It removes 20 read checks and 15 write checks in each matching
body. The complete source algorithm, allocation behavior and ordinary contract
boundary remain unchanged. All 81 complete compression cases are timed and checked.

| Round comparison | Paired geometric elapsed | Existing pooled gates | Existing block gates | Below-reference cases |
|---|---:|---:|---:|---:|
| Repaired R261 to R263 | **−1.188790%** | 81/81 | 81/81 | 22 to 22 |

The next priority is the controlled cases nearest 1x, especially repeated and
periodic inputs near 32 KiB. This round creates no new parity crossing. Negative
time changes below mean faster; ratios below 1x mean faster than the reference.

| Input | Bytes | R261 / Roc | R263 / Roc | Time change | Additional cut to 1x |
|---|---:|---:|---:|---:|---:|
| repeated | 32767 | 1.051163x | 1.020506x | -2.9165% | 2.009% |
| repeated | 32769 | 1.052589x | 1.025549x | -2.5688% | 2.491% |
| repeated | 32768 | 1.055728x | 1.027648x | -2.6598% | 2.690% |
| repeated | 33027 | 1.073708x | 1.041968x | -2.9561% | 4.028% |
| periodic | 32768 | 1.089061x | 1.050873x | -3.5065% | 4.841% |
| periodic | 32767 | 1.092386x | 1.055233x | -3.4011% | 5.234% |
| periodic | 33027 | 1.096717x | 1.061571x | -3.2046% | 5.800% |
| periodic | 32769 | 1.094361x | 1.062217x | -2.9372% | 5.857% |

| Input | Bytes | R261 / Roc | R263 / Roc | Time change |
|---|---:|---:|---:|---:|
| eight-literals | 8 | 0.414859x | 0.420113x | +1.2663% |
| prefix-natural-32768 | 32768 | 1.836182x | 1.832518x | -0.1995% |
| prefix-random-32768 | 32768 | 1.824029x | 1.786578x | -2.0532% |
| prefix-natural-1048576 | 1048576 | 1.902061x | 1.896635x | -0.2852% |
| prefix-random-1048576 | 1048576 | 2.167748x | 2.158164x | -0.4421% |
| prefix-periodic-1048576 | 1048576 | 1.267936x | 1.221557x | -3.6578% |
| prefix-repeated-1048576 | 1048576 | 1.222298x | 1.180133x | -3.4497% |

[All 81 before/after rows, compiler design, exact pins, qualification and evidence](native-guarded-index-bounds.md).
Every historical round below keeps its original denominator. The current comparison
uses two fresh balanced cohorts and ratios formed inside 48 blocks per case.
No valid observation is discarded or rerun and no gate is relaxed. Small adverse
results remain: eight +1.2663%, random 64 +3.9597%, eight-prefix +3.5080%.

All 81 payload allocation/requested-byte/copy/zero/private-peak fields remain
unchanged. Fresh independent instructions decline on natural/random/periodic/
repeated 1 MiB by 1.64%/3.62%/3.35%/3.32%. The worker shrinks 2,100,808→2,084,296 bytes;
matching frames grow 16 bytes and the root frame shrinks 32 bytes. Fresh profiles
retain the path phase as the main natural-text bottleneck.

719 native tests, 3936 linked/resident comparisons and 324 complete original/decoded
smokes pass; seven inherited native exclusions remain. The unchanged maintained
680-test result is inherited. Full CI, the full linked/AOT integration suite and
full 6.6 MB input are not freshly run. The memory-only cohort tail overlaps the
first profile; no isolated RSS improvement is claimed. Primary elapsed and CPU
instruction/cycle measurements finish before profiling. Ordinary managed
admission/accounting/publication/recovery costs remain outside the compact timing
entry, and their integration remains open. IR/work/ABI/bytecode stay 215/107/39/14.

## Historical measurements

R256 composed fixed-buffer nested computations into one guarded native phase.
Its historical all-81 improvement is 18.2622%; R261 repairs preserve that Object.
The tables below remain historical rather than current R263 results.

## R262 packed-state retest

R261 remains retained. The old packed scalar-choice idea is retested after
native outlining: its continuing loop shrinks14→12 instructions with no frame
growth, but the empty outer route grows39→44 through extra stack traffic.
All81 paired elapsed worsens2.8700%; random1MiB changes2.168785x→2.254190x
reference and periodic1MiB1.268420x→1.332019x. The unchanged gates pass76/81
point and77/81 block cases. All81 heap/copy/private-peak fields stay equal.
The candidate is archived; no slower executable replaces the baseline.
[Complete retest, native witnesses, profiles and all81 evidence](native-scalar-choice-retest.md).

## R261 interaction repairs

The four review findings are repaired and qualified. The complete compressor
Object and linked worker remain byte-identical to R256; no new optimization is
enabled. Fresh all81 paired measurements pass every existing point/block 1.05
gate, with identical heap/copy/zero/private-peak fields. The geometric change is
−0.5828%, with the largest slower row +1.1520%; these are measurements of identical
code, not attributable performance effects. Fresh reference multiples are eight
0.417756x; at32KiB text/random/periodic/repeated
1.837612x/1.816686x/1.081951x/1.051936x; at1MiB
1.901775x/2.169105x/1.265729x/1.221235x. The historical R256 table below remains
unchanged. Maintained projection correctness is tested separately; its added
admission cost is not established by compact-entry timings.
[Repairs, native regressions, coverage limits and verified packet](native-review-interactions.md).

## R260 empty-path and fixed-placement control

R256 remains the measurement control. The current empty route shrinks39→21 instructions, and all178 retained function entries stay at the same linked addresses. Text1MiB improves4.11% to1.821964x reference, but random/periodic/repeated regress3.61%/3.68%/4.11% despite fewer instructions. Both controls are archived; the retained table below is unchanged. All81 heap/copy/zero/private-peak fields match. The new review of inherited compiler/runtime/tooling defects takes priority over another optimization. [Full paired all81 results, native census and qualification](native-empty-path-placement-control.md).

## R259 matched proof-use control

R256 remains retained. A fresh four-actor comparison measures R256, explicit
adapter-window gating, fullR257 and the reference together on all81 cases.
The explicit-window control is2.8501% slower by paired geometric elapsed and
fails two combined gates. FullR257 is1.0201% faster in the same cohort, while its
repetitive penalty and historical misses remain. The control adds a redundant
adapter check at source-window-proved sites, so it is not pure geometry-only
attribution. All81 three-way heap/copy fields match. None of these observations
replace the retained table. [All81 paired results and disposition](native-explicit-window-control.md).

## R258 SSA-isolation component

R256 remains the retained compiler. R258 records a1.2245% all81 within-block
geometric elapsed improvement, with text/random gains and periodic/repeated
instruction increases. At1MiB the paired candidate/reference ratios are
1.880496x/2.102709x/1.294166x/1.234041x for text/random/periodic/repeated;
paired changes versus its fresh R256 control are−1.5147%/−2.9981%/+2.1049%/+0.7183%.
Eight is0.424178x in that paired cohort. These are experimental observations,
not replacement retained timings. All combined gates pass; the second-cohort
empty miss remains. New effect tables calculate ratios inside each balanced
block first, with the existing pooled gates preserved.
[Complete component and next exact geometry control](native-packed-read-ssa.md).

## R257 component and timing audit

**R256 remains retained.** The next packed-read geometry experiment is archived with all81 results, including text/random gains, repetitive-input instruction increases and the failed pooled empty timing gate. Code shrinks, but executed work does not improve uniformly. [Complete R257 component, qualification, profiles and all81 observations](native-entry-packed-geometry.md).

The supplemental paired audit finds two R257 sign reversals from pooling actor medians across heterogeneous cohorts. Repeated32KiB's pooled−2.1642% masks individual+0.5778%/+0.3198% slowdowns; comparing within the48 balanced blocks gives+0.3240%. The same audit confirms R256's18.2763% matrix improvement and finds no R256 reversals. Existing observations, pooled ratios and1.05 gates are preserved. The historical R256 table below remains unchanged; pooled medians are descriptive, not paired causal estimates.

## Historical R256 below-reference configurations

| Group | Case | Bytes | R256 / Roc |
|---|---|---:|---:|
| original25 | eight-literals | 8 | **0.414364x** |
| original25 | eight-overlap | 8 | **0.411438x** |
| original25 | 320-overlap | 320 | **0.447221x** |
| original25 | eight-literals-prefix | 8 | **0.411343x** |
| original25 | 128-biased-dynamic | 128 | **0.409098x** |
| original25 | 32-high-entropy-stored | 32 | **0.422530x** |
| original25 | random-64 | 64 | **0.437030x** |
| original25 | random-256 | 256 | **0.475533x** |
| original25 | random-1024 | 1024 | **0.538481x** |
| original25 | repeated-1024 | 1024 | **0.518242x** |
| original25 | reymont-prefix-1024 | 1024 | **0.669349x** |
| prefix56 | prefix-natural-1024 | 1024 | **0.675251x** |
| prefix56 | prefix-random-1024 | 1024 | **0.537245x** |
| prefix56 | prefix-random-4096 | 4096 | **0.947134x** |
| prefix56 | prefix-periodic-1024 | 1024 | **0.522303x** |
| prefix56 | prefix-periodic-4096 | 4096 | **0.765540x** |
| prefix56 | prefix-periodic-8192 | 8192 | **0.875809x** |
| prefix56 | prefix-periodic-16384 | 16384 | **0.980901x** |
| prefix56 | prefix-repeated-1024 | 1024 | **0.521466x** |
| prefix56 | prefix-repeated-4096 | 4096 | **0.741312x** |
| prefix56 | prefix-repeated-8192 | 8192 | **0.851834x** |
| prefix56 | prefix-repeated-16384 | 16384 | **0.949197x** |

## Historical R255 versus R256 matched measurements

Ratios use pooled observation medians. Changes use balanced block means against matched R255. Positive changes are regressions. Instructions/cycles are from a separate cohort. All observations and the three individual Primary B gate misses remain recorded.

| Case | Bytes | R255 / Roc | Retained R256 / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190x | 1.976190x | +4.1367% | +0.3199% |
| one-stored | 1 | 1.976190x | 1.976190x | +3.4691% | +0.3297% |
| seven-stored-prefix | 7 | 1.976190x | 1.976190x | +1.5995% | +0.3271% |
| eight-literals | 8 | 0.427100x | 0.414364x | -3.4678% | -2.9654% |
| eight-overlap | 8 | 0.415224x | 0.411438x | -0.4414% | -2.7329% |
| 320-overlap | 320 | 0.471106x | 0.447221x | -3.7754% | -6.9207% |
| eight-literals-prefix | 8 | 0.419347x | 0.411343x | -1.0045% | -2.9685% |
| 128-biased-dynamic | 128 | 0.430983x | 0.409098x | -4.2641% | -5.8909% |
| 32-high-entropy-stored | 32 | 0.449914x | 0.422530x | -6.6809% | -10.2314% |
| 16384-distribution-change | 16384 | 2.324858x | 1.865948x | -19.7393% | -19.4188% |
| 33027-periodic-window | 33027 | 1.444307x | 1.101319x | -23.7476% | -20.9825% |
| random-64 | 64 | 0.466704x | 0.437030x | -6.7839% | -12.1854% |
| random-256 | 256 | 0.545797x | 0.475533x | -12.2065% | -18.3207% |
| random-1024 | 1024 | 0.647682x | 0.538481x | -15.6985% | -20.9159% |
| repeated-1024 | 1024 | 0.563010x | 0.518242x | -8.0336% | -10.9642% |
| random-32769 | 32769 | 2.523387x | 1.809078x | -28.3075% | -28.0738% |
| repeated-32769 | 32769 | 1.374344x | 1.054159x | -23.2973% | -21.0944% |
| reymont-prefix-1024 | 1024 | 0.802377x | 0.669349x | -16.4497% | -16.6329% |
| reymont-prefix-16384 | 16384 | 2.051386x | 1.757274x | -14.3372% | -15.3122% |
| reymont-prefix-32769 | 32769 | 2.128826x | 1.838233x | -13.6504% | -15.7320% |
| heldout-window-32767 | 32767 | 2.126821x | 1.833910x | -13.7722% | -15.7700% |
| heldout-window-32768 | 32768 | 2.128863x | 1.839295x | -13.6020% | -15.7709% |
| heldout-slides-65537 | 65537 | 2.213204x | 1.902064x | -14.0583% | -15.8723% |
| heldout-random-65537 | 65537 | 2.789476x | 1.975392x | -29.1841% | -30.0327% |
| heldout-mixed-131073 | 131073 | 2.274267x | 1.920398x | -15.5597% | -17.6464% |
| prefix-natural-1024 | 1024 | 0.805746x | 0.675251x | -16.3223% | -16.6298% |
| prefix-natural-4096 | 4096 | 1.558745x | 1.318161x | -15.4344% | -15.8698% |
| prefix-natural-8192 | 8192 | 1.856918x | 1.589342x | -14.4096% | -15.1039% |
| prefix-natural-16384 | 16384 | 2.050931x | 1.762051x | -14.0853% | -15.3119% |
| prefix-natural-32767 | 32767 | 2.131404x | 1.837118x | -13.8071% | -15.7724% |
| prefix-natural-32768 | 32768 | 2.135596x | 1.837344x | -13.9658% | -15.7661% |
| prefix-natural-32769 | 32769 | 2.130578x | 1.835871x | -13.8322% | -15.7383% |
| prefix-natural-33027 | 33027 | 2.132550x | 1.839407x | -13.7461% | -15.7381% |
| prefix-natural-65535 | 65535 | 2.127331x | 1.900385x | -10.6681% | -13.6003% |
| prefix-natural-65536 | 65536 | 2.133617x | 1.908106x | -10.5694% | -13.6004% |
| prefix-natural-65537 | 65537 | 2.125089x | 1.903007x | -10.4505% | -13.5776% |
| prefix-natural-131072 | 131072 | 2.137816x | 1.903422x | -10.9642% | -13.5444% |
| prefix-natural-262144 | 262144 | 2.143698x | 1.908282x | -10.9817% | -13.5030% |
| prefix-natural-1048576 | 1048576 | 2.182609x | 1.908252x | -12.5701% | -14.9740% |
| prefix-random-1024 | 1024 | 0.653959x | 0.537245x | -18.1326% | -21.0675% |
| prefix-random-4096 | 4096 | 1.211736x | 0.947134x | -21.8366% | -23.2881% |
| prefix-random-8192 | 8192 | 1.579874x | 1.166803x | -26.1458% | -25.3070% |
| prefix-random-16384 | 16384 | 2.119998x | 1.541744x | -27.2762% | -26.4495% |
| prefix-random-32767 | 32767 | 2.541201x | 1.826622x | -28.1197% | -28.2248% |
| prefix-random-32768 | 32768 | 2.541539x | 1.828522x | -28.0545% | -28.2249% |
| prefix-random-32769 | 32769 | 2.517614x | 1.813471x | -27.9686% | -28.0679% |
| prefix-random-33027 | 33027 | 2.530290x | 1.811509x | -28.4071% | -28.0961% |
| prefix-random-65535 | 65535 | 2.800912x | 2.000222x | -28.5868% | -29.3728% |
| prefix-random-65536 | 65536 | 2.799132x | 1.993951x | -28.7654% | -29.3728% |
| prefix-random-65537 | 65537 | 2.785346x | 1.994217x | -28.4033% | -29.2923% |
| prefix-random-131072 | 131072 | 2.929944x | 2.081702x | -28.9508% | -30.1548% |
| prefix-random-262144 | 262144 | 2.999684x | 2.188740x | -27.0343% | -29.9102% |
| prefix-random-1048576 | 1048576 | 3.069914x | 2.170161x | -29.3087% | -31.2393% |
| prefix-periodic-1024 | 1024 | 0.577033x | 0.522303x | -9.9486% | -12.0610% |
| prefix-periodic-4096 | 4096 | 0.899704x | 0.765540x | -14.9121% | -17.0162% |
| prefix-periodic-8192 | 8192 | 1.076883x | 0.875809x | -18.6718% | -18.3903% |
| prefix-periodic-16384 | 16384 | 1.267506x | 0.980901x | -22.6117% | -20.7949% |
| prefix-periodic-32767 | 32767 | 1.440622x | 1.092726x | -24.1491% | -21.3065% |
| prefix-periodic-32768 | 32768 | 1.439207x | 1.090658x | -24.2181% | -21.3068% |
| prefix-periodic-32769 | 32769 | 1.435808x | 1.088024x | -24.2222% | -21.1598% |
| prefix-periodic-33027 | 33027 | 1.439778x | 1.098919x | -23.6744% | -20.9820% |
| prefix-periodic-65535 | 65535 | 1.553309x | 1.168122x | -24.7978% | -21.4074% |
| prefix-periodic-65536 | 65536 | 1.554477x | 1.166292x | -24.9721% | -21.4072% |
| prefix-periodic-65537 | 65537 | 1.552722x | 1.166647x | -24.8645% | -21.3321% |
| prefix-periodic-131072 | 131072 | 1.622989x | 1.212178x | -25.3120% | -21.4600% |
| prefix-periodic-262144 | 262144 | 1.663151x | 1.240945x | -25.3859% | -21.5047% |
| prefix-periodic-1048576 | 1048576 | 1.691711x | 1.267271x | -25.0894% | -21.0589% |
| prefix-repeated-1024 | 1024 | 0.562111x | 0.521466x | -6.9248% | -10.9512% |
| prefix-repeated-4096 | 4096 | 0.875654x | 0.741312x | -15.3419% | -16.6166% |
| prefix-repeated-8192 | 8192 | 1.046276x | 0.851834x | -18.5842% | -18.1769% |
| prefix-repeated-16384 | 16384 | 1.214046x | 0.949197x | -21.8154% | -20.6697% |
| prefix-repeated-32767 | 32767 | 1.374672x | 1.053905x | -23.3340% | -21.2420% |
| prefix-repeated-32768 | 32768 | 1.372313x | 1.048871x | -23.5691% | -21.2408% |
| prefix-repeated-32769 | 32769 | 1.370309x | 1.047298x | -23.5721% | -21.0948% |
| prefix-repeated-33027 | 33027 | 1.384601x | 1.075042x | -22.3573% | -20.3153% |
| prefix-repeated-65535 | 65535 | 1.480816x | 1.136948x | -23.2215% | -20.8088% |
| prefix-repeated-65536 | 65536 | 1.479569x | 1.132539x | -23.4548% | -20.8089% |
| prefix-repeated-65537 | 65537 | 1.477570x | 1.130412x | -23.4952% | -20.7362% |
| prefix-repeated-131072 | 131072 | 1.544460x | 1.178993x | -23.6631% | -20.5967% |
| prefix-repeated-262144 | 262144 | 1.569101x | 1.225382x | -21.9055% | -20.2888% |
| prefix-repeated-1048576 | 1048576 | 1.583851x | 1.222946x | -22.7866% | -20.1630% |
