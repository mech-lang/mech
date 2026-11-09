# Packed-cache read source control (R225)

Two local bindings reuse each immutable packed cache/header word instead of
reading it twice. This is the isolated source experiment requested before the
compiler range-proof work. The compiler, allocator, algorithm, allocation sites,
tables, search limits and timing boundaries are unchanged.

Retain this measured variant with its tradeoffs, separately from the original
source used to establish the next compiler experiment. Balanced block means
improve **0.538% geometrically across 25 cases**. Natural text at 32769 bytes
improves 2.291%; periodic and repeated data regress 2.097% and 1.981%. All 25
original 1.05 gates and supplementary block-mean gates pass. No best-of-variants
configuration or across-the-board speedup is claimed.

| Input | Before / Roc | Source control / Roc | Block-mean change |
|---|---:|---:|---:|
| Natural text, 32769 | 2.475034× | 2.418329× | −2.291% |
| Periodic, 33027 | 2.143583× | 2.188531× | +2.097% |
| Repeated, 32769 | 2.005809× | 2.045545× | +1.981% |
| Random, 32769 | 3.031776× | 3.062774× | +1.022% |
| Distribution change, 16384 | 2.993311× | 3.008728× | +0.515% |
| Eight literals | 0.545491× | 0.543362× | +0.663% |
| Natural text, 16384 | 2.441433× | 2.398178× | −1.772% |
| Held-out mixed, 131073 | 2.826392× | 2.848484× | +0.782% |

The ratios are per-call medians; changes use balanced block means. Eight-byte
block means rise despite the lower point median. All 25 distributions, including
the three stored-only sub-microsecond cases, remain in the evidence.

## Semantics and provenance

The match body binds at the original first read. Offset-cost lookup retains its
position before the match-length assertion. The path-position body likewise
binds its header at the first read. There are no intervening writes to either
cache word. Only these two immutable bindings change.

Base: `053ce8eda122263cef2e3f268b0e75e47a54d899`. Experimental compiler:
`3c2fadb93ab8254eef8bcd943bbf3b9c76d2876c`. Maintained compiler:
`42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`, IR/work/ABI/bytecode
215/107/39/14. No maintained compiler or LLVM-backend change.

- Original source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Variant source: `138f42708c3d4e5c1d8ad3b3731988ebe1a0ab5ed46abb3c51b7070a2070079d`.
- Variant bytecode: `4eec1e6eff5c8ba886a3a1991937a8657ec87cf5afaae1eecb79e01ea8987923`.
- Variant Object: `2f30027bd0ef3ca95f00cc36520bf04055863e113936e1a55cfa8fd70be6cc7c`.

The source is encoded as strict revision-14 bytecode, decoded/re-encoded
identically, and compiled from the decoded artifact. A new full original/decoded
machine-image pair is not claimed. Existing R223 matrix controls are historical.

## Measurements and generated work

The R224 fixed-level/runtime-input reference and matched borrowed-output contract
remain. Three persistent workers receive 200 ms warmup per case and use 24
balanced permutations, the same Rust clock and independent inflate validation.
Complete scratch construction/compression/cleanup remain timed. All outputs,
input/output aliases, capacity and prefixes agree through 131073 bytes.

There are 162222 complete primary calls, 162072 retained observations, and 3900
separate CPU/heap calls. Instructions fall on 20 cases, rise on two and tie on
three. Cycles fall on 20 and rise on five. Natural32 instructions fall
165070590→161119189 (−2.394%). Periodic/repeated instructions rise approximately
0.644%/0.648%; retain those contrary results. Random instructions fall 0.616%
while cycles rise 0.775%. These independent cohorts are not instruction shares
of sampled stacks.

All 25 before/after allocation, requested-byte, reallocation, incremental usable
peak and payload/copy/zero fields match. Eight-byte requests remain 230 versus
96 for the matched reference. Natural32 requests remain 608 versus 277.
Requested capacity, initialized copies, incremental usable peak, full RSS and
stack are different measurements. Both hold the complete input; representation
preparation is outside timing. Mech's full RSS includes large parsed diagnostic
metadata and must not be presented as an input-copy ratio.

Four path bodies shrink 2208→2160 bytes each. Actual disassembly shows two fewer
scalar U32 loads and two fewer conditional branches. The three matching bodies
remain byte-identical. Root text shrinks 26660 bytes; root stack grows 16 bytes
to 86784. The same 134 numeric phases and two maps remain selected.

Fresh wall and current-PC captures execute 24001 and 91109 correct calls.
Natural, periodic, repeated and random have approximately 95–97% generated
exclusive wall weight. Distribution change has 79.154% generated, 5.059% host
and 15.787% other worker/system weight; that contrary capture stays. Eight-byte
weight is 42.361% generated and 34.332% host. PC samples resolve the current
Object, not historical offsets. Sampling percentages are not removable-time
predictions or retired-instruction counts.

## Failures, replay and next step

The first controller hit actual OS disk exhaustion after 19 complete cases.
Their hashes, partial next case and original failure logs remain. Only six
missing cases resumed after verified disposable archive extractions were freed;
the original 19 are byte-identical. Interrupted workers have no graceful final
cleanup claim. Completed calls, resumed workers, counter and profile cohorts
check cleanup. The memory-probe build initially lacked its existing usage-reader
Object; that recorded linker failure is corrected without a compiler change.
No program allowance or expected output changes.

No fresh maintained 678-test suite, ordinary linked qualification, all-feature
build, complete phase-observer census or multi-megabyte execution is claimed.
The same seven inherited normal exclusions remain. The standalone experiment
continues to omit the additional ordinary resource/admission/publication/recovery
contract; its restoration is not a prerequisite for this comparison.

Next: diagnose actual child-reduction proof refusals on the **original source**,
then preserve checked match-length, cost-table and growing-node length facts
with checked fallback and fresh pointers. Measure that compiler change separately
before combining this source control. Private-write simplification, shared phase
packet storage and wider matching remain later independent experiments. The full
brief and parity remain open.

## Evidence

[r225-cache-word-source-08b13b09e6ee.tar.gz](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r225-cache-word-source-08b13b09e6ee.tar.gz), SHA-256
`08b13b09e6eeaaee9fdd0fe99ad4b18058bae126f1d09f917d5c2075f30c1a91`
(2431733 bytes; 396 hashed files). Local and downloaded archives pass
all file hashes and byte-identical numerical replay. `REPORT.md`, `source.patch`,
clients/build specifications, all 25 raw structured observations, independent
counter/profile cohorts and failure/recovery receipts are archived. Native
binaries, bytecode, full executable metadata and raw traces are excluded.

The original packet retained the complete delta but its extension filter omitted
the full Mech source. The [exact measured source](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r225-cache-word-source-138f42708c3d.mec) and its companion hash receipt are now supplied separately
and verified on the server and after download. Existing archives and results are
unchanged. R226 explicitly checks inclusion of all candidate Rust and measured
Mech source files.
