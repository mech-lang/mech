# R257: reuse current-entry geometry for packed reads

**Archive this component and retain R256.** All81 balanced geometric elapsed change -0.7560%; larger controlled prefixes -0.4995%. Parity remains incomplete. The supplemental within-block geometric elapsed change is -0.8009%. The original pooled gates remain authoritative and unchanged.

Git parent `608ed69aedb98bb8c9d61b0bf6364580b304a6d1`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Existing draft #973 and its one active branch are retained. Two compiler files change and one source fixture is added. All826 current source files reconstruct exactly from the825-file R256 base. Retained R255 Cranelift0.131.3 backend; ordinary IR/work/ABI/MBC215/107/39/14 unchanged.

## Design

R256 current-image samples exposed repeated row/column loads and unit-geometry tests in the collector’s packed-read adapter. Tracing canonical sites1056/1062/1063 established that the hot sequence occurs before a child version: checked length-minus-four, current-view geometry, then the packed load. The old adapter took its flat-shape argument only from a complete child-window proof. The current parent entry’s actual VectorContract was ignored. This repairs that specific disconnect; it is not a general child-fact propagation pass.

With `MECH_CURRENT_PACK_GEOMETRY=1`, Compact entry geometry comes only from existing phase-entry bounds and the caller-constructed VectorContract. Arrays with appends are excluded. The shared numeric provenance requires all array definitions to identify the same fixed carrier buffer; indexed writes preserve geometry. The selected pack must remain confined to its current numeric region/version. No pointer, length interval, owner, alias or current-account permission is imported. A cold export ends the call and the next entry checks current views afresh.

Geometry and readable-window safety are independent. A geometry-only pack keeps its original checked arithmetic/load path and dynamic range gate. Complete replacement additionally requires the existing fresh source-window proof: current logical length, just-executed checked subtraction, selected comparison edge, no unsupported action, boundary or stale definition between them. The source checked subtraction and branch remain. General matrix views retain the original native function and canonical linear_ordinal mapping. The ordinary funded entry is unchanged; default-off reproduces the entire R256 Object and metadata exactly.

The source program, bytecode, level12 algorithm, tables, depth300, nice258, near-EOF limits, source allocation sites, real262144-U16 tree,1500513-U32 cache,305257-U64 reservation, output clear/reuse, allocator and complete-call timing boundary are pinned. This remains one executable accepting runtime input for every case. The compact entry’s extra omitted exact managed admission/accounting/resource/publication/recovery contract is not restored by these tests.

## Observed mechanism

Only `mech_compact_numeric_837_vector` changes among340 numeric definitions;339 retain exact code bytes and stack metadata. No function is added or removed. Its body42800→23588bytes and stack928→816bytes. Object text1385816→1366616bytes; worker2100808→2067784bytes. Root code1040856bytes and frame57088bytes remain unchanged.

Twenty-seven newly selected packed-read sites reuse current-entry geometry; all are before a child version and have an independent source-window proof. These are static eligible sites, not an exact dynamic visit census. The previous vector path already used a packed load on successful vector reads: much of the code reduction is unexecuted checked fallback bodies. Executed instruction savings are measured separately below.

The new source-shaped tests include both guarded and unguarded packs. All eight initial four-byte candidate original/decoded runs actually enter the selected vector function; guarded variants have144 vector entries each and unguarded variants72 each. Full operation counts remain source-dependent. C and Rust safe-window structural witnesses each compile to a single four-byte load after the source checks, with no call instruction. They are small compiler witnesses, not additional compression benchmarks.

## Matched complete-call results

Two fixed24-block cohorts retain all six balanced actor orders, matched200ms warmup, runtime input and fixed level12 on both sides, matched live borrowed output, clock, allocator and validation placement. Full compression, scratch construction, transfer and private cleanup are timed; validation stays outside. No build, test, profile or other diagnostic compressor overlaps a primary/CPU/heap cohort. Hardware and heap are independent cohorts; sampled-time weights are not instruction partitions.

| Case | Mech ms | Roc ms | R256 / Roc | R257 / Roc | Pooled descriptive elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.119542 | 0.290042 | 0.411868× | 0.412154× | -1.5462% | -0.0130% |
| prefix-natural-32768 | 7.898500 | 4.343604 | 1.842940× | 1.818421× | -1.3304% | -2.5112% |
| prefix-natural-1048576 | 303.680229 | 161.075250 | 1.909334× | 1.885331× | -1.2571% | -2.5431% |
| prefix-random-32768 | 1.866417 | 1.054624 | 1.830272× | 1.769745× | -3.3070% | -4.1126% |
| prefix-random-1048576 | 60.536855 | 28.753542 | 2.172545× | 2.105370× | -3.0920% | -4.9512% |
| prefix-periodic-32768 | 1.363687 | 1.241125 | 1.087555× | 1.098751× | +1.0295% | +1.4504% |
| prefix-periodic-1048576 | 42.750166 | 33.009833 | 1.271069× | 1.295074× | +1.8885% | +1.4784% |
| prefix-repeated-32768 | 1.493709 | 1.414854 | 1.079087× | 1.055733× | -2.1642% | +1.4593% |
| prefix-repeated-1048576 | 42.227271 | 34.247104 | 1.222486× | 1.233017× | +0.8615% | +1.4620% |

Pooled point gates81/81; pooled balanced gates80/81, with the unchanged1.05 allowance. Balanced improvements53; adverse cases28. Raw improvements49; ties3. Individual cohorts: primary-a point81/81, balanced81/81; primary-b point81/81, balanced80/81.

**Every individual gate miss:** [{"cohort": "primary-b", "case": "empty", "point_ratio": 1.0, "balanced_ratio": 1.073711375212224, "point_gate": true, "balanced_gate": false}]. Contrary individual/cohort timing pairs and every adverse case remain in `primary-results.json` and `result-decision.json`; no allowance or expected value changes. Historical R256 results retain their original denominators; this round uses a fresh matched reference.

## Instructions and memory

Independent CPU cohort: 46 instruction decreases, 32 increases; 57 cycle decreases, 24 increases. Tiny counter differences are retained without a fresh probe-overhead subtraction or unsupported source-level attribution.

| Case | Mech instructions | Roc instructions | Mech / Roc instructions | Mech / Roc cycles | Mech private peak MB | Mech / Roc heap requests |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 1.13719e+06 | 1.36193e+06 | 0.834984× | 0.528542× | 8.982372 | 230 / 96 |
| prefix-natural-32768 | 1.03502e+08 | 4.76142e+07 | 2.173767× | 1.822333× | 9.015175 | 606 / 277 |
| prefix-random-32768 | 3.06344e+07 | 1.14536e+07 | 2.674646× | 1.776497× | 9.008351 | 274 / 115 |
| prefix-periodic-32768 | 2.90788e+07 | 1.91554e+07 | 1.518045× | 1.102624× | 9.011274 | 206 / 84 |
| prefix-repeated-32768 | 2.90239e+07 | 1.91521e+07 | 1.515438× | 1.069725× | 9.011274 | 206 / 84 |
| prefix-natural-1048576 | 3.74981e+09 | 1.67071e+09 | 2.244445× | 1.888870× | 10.550358 | 1556 / 695 |
| prefix-random-1048576 | 9.39155e+08 | 3.20166e+08 | 2.933336× | 2.101141× | 10.550486 | 914 / 373 |
| prefix-periodic-1048576 | 9.10053e+08 | 5.75958e+08 | 1.580068× | 1.282848× | 10.550486 | 742 / 291 |
| prefix-repeated-1048576 | 9.17123e+08 | 5.76026e+08 | 1.592156× | 1.235187× | 10.550486 | 742 / 291 |

All81 payload/copy/zero/private-peak and heap medians identical: **True**. Per-field deltas, requested bytes, reallocations and initialized copied bytes are retained for every case. Requested capacity is not copied/live memory. Private payload excludes prepared input and code/stack; RSS/physical/lifetime fields are process observations, not precise per-call live allocation or memcpy volume. The current observed reference heap request count is an implementation result, not a semantic minimum.

## Current-image profiles

Six fresh5-second captures per executable cover eight bytes,32KiB text/random, and1MiB text/random/periodic. Profiles include validation/harness and retain actual hashed reference symbols.

| Mech case | Running samples | Root | Numeric | Other/external |
|---|---:|---:|---:|---:|
| eight-literals | 5536 | 837 | 711 | 3988 |
| prefix-natural-1048576 | 5383 | 63 | 5261 | 59 |
| prefix-natural-32768 | 5536 | 136 | 5289 | 111 |
| prefix-periodic-1048576 | 5562 | 80 | 5314 | 168 |
| prefix-random-1048576 | 5604 | 12 | 5515 | 77 |
| prefix-random-32768 | 5566 | 129 | 5177 | 260 |

| Reference case | Running samples | Generated compression | Builtin/ownership | Other/external |
|---|---:|---:|---:|---:|
| eight-literals | 5368 | 5213 | 3 | 152 |
| prefix-natural-1048576 | 5389 | 5305 | 0 | 84 |
| prefix-natural-32768 | 5418 | 5277 | 2 | 139 |
| prefix-periodic-1048576 | 5399 | 5320 | 0 | 79 |
| prefix-random-1048576 | 5391 | 5345 | 0 | 46 |
| prefix-random-32768 | 5375 | 5309 | 1 | 65 |

Current aggregate symbol/PC weights, exact artifact pins and narrow actual-image assembly windows are retained. Reference hashed names are not assigned unverified source names. This small geometry repair does not remove the remaining path-scan computation or its larger-input cost. Before combining this component, isolate shape-only reuse from complete fallback removal and inspect the actual periodic skip-loop moves/spills; the observed instruction increase prevents treating smaller code as lower executed work. Then return to the path-scan target. Preserve both fixed cohorts; do not rerun solely to hide the empty gate miss. Broad identity forwarding from R252/R254 remains rejected; R247 single-placement and empty outer reduction are still open.

## Qualification and reproduction

- `python3 run-spec.py candidate-gpu candidate-producer`: retained native feature profile, opt3 and warnings-as-errors. Exact commands/environment/dependencies and binary hashes are in the build receipts. No whole math-library rebuild.
- `python3 run.py pack-qualification python3 qualify-packs.py`:67200 actual linked calls across four source variants, parent/candidate and original/decoded. Independent canonical matrix oracle; exact status, source origin and result equality. Covers zero trips, row/column vectors,2D matrices, empty/short buffers, extra physical canary capacity, arithmetic overflow versus bounds/assertion precedence, live old buffers, current arguments, unchanged input aliases/prior publication, failed output sentinels, cleanup and recovery. Every run ends at zero private bytes/owners.
- `python3 run.py broad-qualification python3 qualify-broad.py`:715 maintained tests pass,0fail/ignored,7 unchanged inherited exclusions. Full default-off Object/metadata equals R256; full annotated current Object/metadata equals the timed image. The new source fixture is part of the826-file reproducible compiler closure.
- `python3 run.py all81-immediate-smokes python3 smoke-all.py`:162 complete golden/inflate/alias/capacity/cleanup calls.
- `python3 run.py primary-a-measurement python3 measure-primary-a.py`; `python3 measure-rest.py`: all81 elapsed twice, independent CPU/heap once each; exact raw observations retained losslessly. Old completion-label wording in the inherited cohort templates is retained; exact worker/Object hashes identify this geometry candidate.
- `python3 run.py current-pc-profile python3 profile-compact-pcs.py after`; same with `reference`:12 new exact-image captures, separate from timings/counters.
- `python3 inspect-machine.py`; `python3 pin-source.py`; `python3 verify-source.py`: function bytes,826→825→826 exact source reconstruction and all source/bytecode/backend/allocator pins.
- `python3 replay.py`: six reports replay byte-identically from archived observations, without running a compiler/compressor.

The prior full36,864 cold controls,3500 source controls, extended R223/R250 matrix/buffer suites and eleven-category algorithmic census are inherited, not falsely counted as new. Additional full decoded compressor images are not emitted; the complete benchmark uses the pinned decoded MBC14 input and all four focused fixtures execute original and decoded artifacts. No6.6MB timing is attempted. Hosted CI is cancelled only at this exact head under the standing instruction; review workflows are preserved and the PR stays draft.

Setup corrections retained: the initial inventory-only call used unsupported header selector `all`; the corrected call removes it. The initial fixture hit the existing escaping-result refusal; projecting the private record through the public tuple wrapper selects the existing supported phase. No compiler refusal was relaxed. No new semantic failure is hidden. All inherited exclusions and timing misses remain explicit.

## Paired timing audit

The existing pooled summary takes separate actor medians across both cohorts and divides them. That descriptive statistic can reverse both within-cohort directions. It must not be treated as a paired causal estimate. R257 has two strict sign reversals: `prefix-repeated-32768` and `prefix-periodic-131072`.

For repeated32KiB the pooled figure says−2.1642%, while cohort A says+0.5778% and cohort B+0.3198%. The mean log after/before ratio of the48 balanced measurement blocks gives+0.3240%. Both original cohorts, every raw observation and the original1.05 gate are retained. No extra run or allowance change hides the pooled empty gate failure(+5.6582%; second cohort+7.3711%;83ns raw medians unchanged).

Supplemental `analyze-paired.py` computes each after/before and after/reference ratio within its existing balanced block, then reports geometric means and medians. It does not replace or rewrite primary measurements or gates. Historical R256 is audited from exact extracted blocks with original observation hashes: its paired all81 effect is−18.2763%, close to its published pooled−18.2622%, with no strict sign reversals. No audit of all older rounds is claimed.

| 32KiB family | Pooled descriptive candidate / Roc | Within-block candidate / Roc | Within-block elapsed change | Instructions change |
|---|---:|---:|---:|---:|
| prefix-natural-32768 | 1.818421× | 1.815773× | -1.2906% | -2.5112% |
| prefix-random-32768 | 1.769745× | 1.763275× | -3.0818% | -4.1126% |
| prefix-periodic-32768 | 1.098751× | 1.102124× | +1.3634% | +1.4504% |
| prefix-repeated-32768 | 1.055733× | 1.062341× | +0.3240% | +1.4593% |

## Additional width qualification

`python3 run.py pack-width-qualification python3 qualify-widths.py` adds134400 linked width2/8 calls. `python3 run.py pack-wide-qualification python3 qualify-wide.py` adds67200 width8 controls with16-byte vectors. The latter explicitly reaches a native vector phase while three further eight-byte windows remain readable, including live old arrays and original/decoded forms; merely entering on the final no-work visit would not establish that. Status/origin/results match the independent oracle and the parent in every run. All three harness sources and generated fixtures are archived without overwriting the initial four-byte receipts. Total268800 calls are executions, not268800 unique test definitions.

## All81 candidate observations

The retained compiler remains R256. These are archived R257 observations. Ratios are the existing pooled point medians; the added paired column summarizes within-block ratios. Positive changes are regressions. Hardware counters are independent.

| Case | Bytes | R256 / Roc | R257 / Roc | Paired elapsed change | Pooled descriptive change | Instructions change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 1.976190× | 1.976190× | +4.5846% | +5.6582% | +0.0000% |
| one-stored | 1 | 1.976190× | 1.976190× | +1.8323% | +2.4766% | +0.0000% |
| seven-stored-prefix | 7 | 1.976190× | 1.976190× | -0.0044% | -0.3064% | +0.0000% |
| eight-literals | 8 | 0.411868× | 0.412154× | -0.4417% | -1.5462% | -0.0130% |
| eight-overlap | 8 | 0.408667× | 0.412554× | +0.2627% | +0.1828% | -0.0074% |
| 320-overlap | 320 | 0.446565× | 0.446568× | -0.2851% | -0.3359% | +0.3971% |
| eight-literals-prefix | 8 | 0.412724× | 0.411862× | -1.4427% | -1.6313% | -0.0133% |
| 128-biased-dynamic | 128 | 0.411832× | 0.409818× | -1.3335% | -0.0502% | -0.3450% |
| 32-high-entropy-stored | 32 | 0.421113× | 0.427105× | +1.8777% | +1.7658% | -0.0685% |
| 16384-distribution-change | 16384 | 1.878781× | 1.830364× | -2.3910% | -2.5771% | -2.7789% |
| 33027-periodic-window | 33027 | 1.093043× | 1.101022× | +0.7725% | +0.7300% | +1.4327% |
| random-64 | 64 | 0.426871× | 0.426585× | -0.9182% | -0.2745% | -0.1415% |
| random-256 | 256 | 0.467777× | 0.468467× | -0.8213% | -0.8654% | -0.4334% |
| random-1024 | 1024 | 0.533478× | 0.525275× | -1.6977% | -2.0570% | -1.4760% |
| repeated-1024 | 1024 | 0.515263× | 0.509195× | -1.5945% | -1.4943% | +0.6638% |
| random-32769 | 32769 | 1.810327× | 1.748678× | -3.2739% | -3.4054% | -4.0865% |
| repeated-32769 | 32769 | 1.054378× | 1.060537× | +0.5774% | +0.5841% | +1.4472% |
| reymont-prefix-1024 | 1024 | 0.671340× | 0.665743× | -1.3111% | -1.5478% | -1.6287% |
| reymont-prefix-16384 | 16384 | 1.769245× | 1.736279× | -1.7909% | -1.8633% | -2.2743% |
| reymont-prefix-32769 | 32769 | 1.839626× | 1.816348× | -1.2364% | -1.2654% | -2.5052% |
| heldout-window-32767 | 32767 | 1.844175× | 1.822299× | -1.2550% | -1.1862% | -2.5115% |
| heldout-window-32768 | 32768 | 1.845481× | 1.824318× | -1.1193% | -1.1467% | -2.5112% |
| heldout-slides-65537 | 65537 | 1.905233× | 1.877280× | -1.5614% | -1.4672% | -2.6259% |
| heldout-random-65537 | 65537 | 1.982587× | 1.914628× | -3.1935% | -3.4278% | -4.6073% |
| heldout-mixed-131073 | 131073 | 1.921552× | 1.893173× | -1.3860% | -1.4769% | -2.0579% |
| prefix-natural-1024 | 1024 | 0.672098× | 0.666791× | -0.9412% | -0.6494% | -1.6293% |
| prefix-natural-4096 | 4096 | 1.337372× | 1.306968× | -2.4183% | -2.2734% | -2.1879% |
| prefix-natural-8192 | 8192 | 1.600883× | 1.574483× | -1.7713% | -1.6491% | -2.1730% |
| prefix-natural-16384 | 16384 | 1.765270× | 1.739539× | -1.4878% | -1.4576% | -2.2742% |
| prefix-natural-32767 | 32767 | 1.845328× | 1.820629× | -1.1880% | -1.3385% | -2.5113% |
| prefix-natural-32768 | 32768 | 1.842940× | 1.818421× | -1.2906% | -1.3304% | -2.5112% |
| prefix-natural-32769 | 32769 | 1.847686× | 1.820707× | -1.5337% | -1.4601% | -2.5053% |
| prefix-natural-33027 | 33027 | 1.841441× | 1.814665× | -1.4430% | -1.4541% | -2.5058% |
| prefix-natural-65535 | 65535 | 1.910471× | 1.886502× | -1.1927% | -1.2546% | -2.2515% |
| prefix-natural-65536 | 65536 | 1.907961× | 1.889662× | -0.9971% | -0.9591% | -2.2521% |
| prefix-natural-65537 | 65537 | 1.907763× | 1.886810× | -0.7508% | -1.0983% | -2.2517% |
| prefix-natural-131072 | 131072 | 1.903354× | 1.881522× | -1.1566% | -1.1471% | -2.2547% |
| prefix-natural-262144 | 262144 | 1.908369× | 1.889431× | -1.0454% | -0.9924% | -2.2535% |
| prefix-natural-1048576 | 1048576 | 1.909334× | 1.885331× | -1.1339% | -1.2571% | -2.5431% |
| prefix-random-1024 | 1024 | 0.535089× | 0.531862× | -0.0319% | -0.7574% | -1.4851% |
| prefix-random-4096 | 4096 | 0.968652× | 0.935690× | -2.4110% | -3.4028% | -2.7125% |
| prefix-random-8192 | 8192 | 1.206381× | 1.145188× | -4.9723% | -5.0724% | -3.3285% |
| prefix-random-16384 | 16384 | 1.561755× | 1.512992× | -2.5873% | -3.1223% | -3.6377% |
| prefix-random-32767 | 32767 | 1.821058× | 1.766882× | -3.2922% | -2.9750% | -4.1128% |
| prefix-random-32768 | 32768 | 1.830272× | 1.769745× | -3.0818% | -3.3070% | -4.1126% |
| prefix-random-32769 | 32769 | 1.811002× | 1.756845× | -3.5311% | -2.9905% | -4.0815% |
| prefix-random-33027 | 33027 | 1.825488× | 1.763775× | -3.2565% | -3.3806% | -4.0864% |
| prefix-random-65535 | 65535 | 2.002228× | 1.934648× | -3.4646% | -3.3753% | -4.4576% |
| prefix-random-65536 | 65536 | 2.000222× | 1.934407× | -3.3826% | -3.2904% | -4.4575% |
| prefix-random-65537 | 65537 | 2.005557× | 1.940034× | -3.1785% | -3.2671% | -4.4407% |
| prefix-random-131072 | 131072 | 2.082360× | 2.023928× | -2.7736% | -2.8060% | -4.6073% |
| prefix-random-262144 | 262144 | 2.188149× | 2.128012× | -2.8962% | -2.7483% | -4.6798% |
| prefix-random-1048576 | 1048576 | 2.172545× | 2.105370× | -3.1490% | -3.0920% | -4.9512% |
| prefix-periodic-1024 | 1024 | 0.521564× | 0.522072× | +0.1990% | +0.1895% | +0.5657% |
| prefix-periodic-4096 | 4096 | 0.761391× | 0.749249× | -1.5605% | -1.5947% | +1.0298% |
| prefix-periodic-8192 | 8192 | 0.876713× | 0.869045× | -0.0623% | -0.8746% | +1.1735% |
| prefix-periodic-16384 | 16384 | 0.982073× | 0.984375× | +0.0619% | +0.2345% | +1.3922% |
| prefix-periodic-32767 | 32767 | 1.089472× | 1.097826× | +0.0852% | +0.7667% | +1.4503% |
| prefix-periodic-32768 | 32768 | 1.087555× | 1.098751× | +1.3634% | +1.0295% | +1.4504% |
| prefix-periodic-32769 | 32769 | 1.089192× | 1.089308× | +0.1802% | +0.0106% | +1.4380% |
| prefix-periodic-33027 | 33027 | 1.091252× | 1.098554× | +0.4458% | +0.6692% | +1.4328% |
| prefix-periodic-65535 | 65535 | 1.167025× | 1.175978× | +0.6803% | +0.7672% | +1.4720% |
| prefix-periodic-65536 | 65536 | 1.170874× | 1.177797× | +0.7993% | +0.5913% | +1.4719% |
| prefix-periodic-65537 | 65537 | 1.166777× | 1.189296× | +1.0921% | +1.9300% | +1.4656% |
| prefix-periodic-131072 | 131072 | 1.226066× | 1.220128× | +0.3553% | -0.4843% | +1.4831% |
| prefix-periodic-262144 | 262144 | 1.251524× | 1.284571× | +1.6586% | +2.6405% | +1.4910% |
| prefix-periodic-1048576 | 1048576 | 1.271069× | 1.295074× | +1.0937% | +1.8885% | +1.4784% |
| prefix-repeated-1024 | 1024 | 0.574972× | 0.563466× | -0.8937% | -4.1027% | +0.6614% |
| prefix-repeated-4096 | 4096 | 0.751330× | 0.769069× | +1.5520% | +2.3610% | +1.0761% |
| prefix-repeated-8192 | 8192 | 0.879295× | 0.868961× | -0.5873% | -1.1753% | +1.2001% |
| prefix-repeated-16384 | 16384 | 0.930981× | 0.958056× | +0.8598% | +2.9083% | +1.4091% |
| prefix-repeated-32767 | 32767 | 1.075680× | 1.081301× | +0.4458% | +0.5226% | +1.4593% |
| prefix-repeated-32768 | 32768 | 1.079087× | 1.055733× | +0.3240% | -2.1642% | +1.4593% |
| prefix-repeated-32769 | 32769 | 1.071144× | 1.101701× | +1.0318% | +2.8528% | +1.4470% |
| prefix-repeated-33027 | 33027 | 1.081040× | 1.115445× | -0.1263% | +3.1826% | +1.4157% |
| prefix-repeated-65535 | 65535 | 1.148037× | 1.173053× | +1.0955% | +2.1791% | +1.4613% |
| prefix-repeated-65536 | 65536 | 1.144307× | 1.149858× | +0.3362% | +0.4851% | +1.4611% |
| prefix-repeated-65537 | 65537 | 1.141159× | 1.156740× | +0.7278% | +1.3654% | +1.4548% |
| prefix-repeated-131072 | 131072 | 1.195107× | 1.204134× | +0.7439% | +0.7554% | +1.4620% |
| prefix-repeated-262144 | 262144 | 1.244631× | 1.256716× | +0.8712% | +0.9710% | +1.4543% |
| prefix-repeated-1048576 | 1048576 | 1.222486× | 1.233017× | +0.9585% | +0.8615% | +1.4620% |

## Pins

- bytecode: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- benchmark_source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- candidate_object: `540deeb9977aad17d607760c664781989de8b457b1983fecd88e4cfa3a43b309`.
- candidate_metadata: `4568703ff36c5db2014b843a9983660d261b7cf335c5c44bf35cbf0f2282ddea`.
- candidate_worker: `7108d3d5a31a18dc32d6ad3df6f69926ac6d7587dfc8da053e89523883d5afc2`.
- compiler_source_manifest: `74ddfd1c8ba9d3040b0d17cd42530206676175ade95c0240d0e83ac3cb92d5ee`.
- compiler_patch: `2498dce0e6cc31e4366e2f43efc9c6e56645e4d33d0240e867cfd101389eabd7`.
- retained_backend_patch: `83c5fc6c14689ba80238615ae6e75c00103cbc447c8ae6681a62fae97607dc54`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- allocator_object: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

## Published checkpoint

The retained compiler remains R256 at source checkpoint `608ed69aedb98bb8c9d61b0bf6364580b304a6d1`; this round archives the component and records its tradeoffs. The primary gains on text/random are preserved as experimental evidence. The component is not rejected merely because its gains are small: periodic/repeated instructions increase and the existing pooled empty timing gate fails. No retiming, expected-value change or increased allowance removes those outcomes.

[Verified evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r257-pack-geometry-d00759469e51.tar.gz) — SHA-256 `d00759469e51061d5f73a537251e5d21354f99fdf6f8e18464f2997149d7f24c`, 21,617,354 bytes, 2,619 files. A fresh public download verifies every file, all826 current source hashes, exact825-file reverse reconstruction, and seven reports replayed byte-identically. Native binaries, bytecode, complete compressor descriptors/maps and raw traces remain excluded. The reversible compiler patch and complete compiler source are inside the archive.

An environmental packaging failure occurred after every measurement and capture completed: local archive extraction ran out of disk space (`OSError: Errno28`). Removing obsolete completed-test binaries (287,213,280 bytes), a disabled-feature backend build (202,406,400 bytes), and the incomplete extraction resolved it. The retained backend, source, measured workers, producers and receipts remain. Both local verification and fresh public verification subsequently pass. A helper-generation guard also stopped before a file write; the corrected checked generator and all width controls pass. These are setup/environmental failures, not compressor semantic failures.

The seven maintained-suite exclusions are inherited coverage exclusions. No newly executed semantic test fails. The new candidate-associated performance failures and the statistical sign reversals remain explicit above. Review stays on the existing draft #973; no branch or PR is created.
