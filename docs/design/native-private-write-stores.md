# Existing-private typed stores (R228)

Retain this compiler experiment over R227. Reuse the exact current private flag
to bypass the old-value load/comparison on already-private unsigned cells.
Shared/no-change cells keep their owner and first-change path. Incoming checks,
bounds, canonical matrix translation, narrowing, resource charges, aliases,
error order and recovery remain. No source algorithm or new ownership proof.

All25 unchanged timing gates pass; geometric balanced block means improve
**0.995%**. Ratios below use matched per-call medians; changes use block means.

| Input | R227 / Roc | R228 / Roc | Block-mean change |
|---|---:|---:|---:|
| reymont-prefix-32769 | 2.358168× | 2.351733× | -0.273% |
| 33027-periodic-window | 2.144714× | 2.126537× | -0.848% |
| repeated-32769 | 2.015390× | 2.009379× | -0.298% |
| random-32769 | 3.021212× | 3.000196× | -0.696% |
| 16384-distribution-change | 2.958124× | 2.929517× | -0.967% |
| eight-literals | 0.519345× | 0.507538× | -2.405% |
| reymont-prefix-16384 | 2.329513× | 2.308411× | -0.906% |

Empty, one-stored and eight-overlap block means rise3.124%,2.813%,0.067%.
Instructions fall on16 cases/tie onthree/rise onsix; natural32 rises0.417%
(+636605 instructions). Cycles fall onall25. All25 allocator, initialized-copy,
zeroing and incremental live-peak metrics match. Preserve these tradeoffs.

## Actual implementation and qualification

Only the archived `program/codegen/readonly_numeric.rs` and
`program/compact/phases.rs` change. Report236 private write sites across130
routines.111 numeric bodies lose3100B total;23 others/two maps stay identical.
The four path scans and prior histogram stay identical. Root text/stack remain
6781056B/86736B; full worker remains7762616B. No per-write dynamic census claim.

684 maintained tests and26 included focused proof tests pass under the same
six-feature warnings-as-errors profile; the same seven inherited exclusions stay.
3400 compiled and2652 ordinary-native controls compare parent/candidate and
original/decoded values, errors, origins, work, aliases, finite clone denial,
prior publication, cleanup and retry. All final managed bytes/nodes are zero.
New all-four-width change-then-equal witnesses actually enter native. The retained
U32-alias witness takes fallback and native at the same header in40 cases.
34 small original/decoded Object/metadata pairs agree within each compiler.

162222 primary calls/3900 separate counter calls,50 immediate full streams and
fresh24195 wall/99444 current-PC calls verify exact streams, independent inflate,
held input/output aliases, original capacities and cleanup. All20 originals and
five held-outs through131073B remain. Fixed level12, runtime inputs, equal200ms
warmup, balanced ordering, allocator, clock and timing boundaries stay matched.
Large wall profiles remain93.82–97.24% generated-exclusive; these are not retired
instruction shares or removable overhead. Full25 distributions are in the packet.

One initial test selector matched zero and was rejected as coverage; the correct
`traversal_facts::` filter executes26 passing tests. A valid nonzero check operand
extends an independent oracle; prior0/MAX cases remain valid. Old temporary input
files are restored from retained hash-checked bytes. No new compiler/compressor
failure, expected-value change or allowance increase. Hosted CI is cancelled by
standing direction, not reported green.

## Eight-byte profile and executable size

The packet includes the requested R227 matched comparison and supplementary
reference/Mech wall profiles:150.292/289.459µs=0.519218× reference; instructions
1.394941m/1.361820m. Two reference initialization loops account for1.142106m
instructions by fixed assembly bounds, about83.87% of its whole-call median;
Mech uses bulk system zeroing. This is an analytical count, not a phase counter.

One executable per implementation handles all25 runtime inputs/sizes. Separate
builds are compiler variants, instrumentation and correctness fixtures. The
6.78MB Mech entry is87.36% of its7.76MB worker; it retains the expanded control
and fallback graph. Reference worker is0.758MB. The footprint gap is generated
code, not a need for a size-specific compressor or a giant linked math library.
RSS includes runtime/harness/diagnostic metadata; compressor heap peaks are
comparable. Neither native/host sample fractions nor file size prove latency.

The next bounded experiment is synchronous phase-call packet lifetime/reuse.
Current268 explicit slots total32168B; the largest input/output pair is512B.
The theoretical31656B saving is not implemented/proved and is not a speed promise.
Wider matching and combining R225 source bindings remain separate.

## Pins and public evidence

Base `5c0a687e77de2ed2bbff998329988da3713a289b`; normal compiler `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5` (IR/work/ABI/MBC215/107/39/14). Normal source is unchanged; the standalone experiment still omits its extra ordinary resource/admission/accounting/publication/recovery contract. Two archived Rust files change; compressor source/allocation sites remain pinned.

Compiler manifest `0fcbc0e659854015bda575385fbdb7569b70fdcbb660b11cd2f59a6ad83379bd`; Object `6766db4860d2697b7d9312cc609246ca199bf93dbe4a7f18762883b06695820a`. Original source/MBC `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.

[Full source, minimal patch, all25 timings/counters/profiles, qualification and comparison supplement](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r228-private-typed-stores-d01b6eb0c244.tar.gz) — 4749208B; 2151 hashed files; SHA-256 `d01b6eb0c2446dee008ebba070ba8f411b9487236033469f57d84cf0cc3ba127`. Local and downloaded verification/replay are recorded in the publication receipt. Native binaries, bytecode, executable metadata and raw traces are excluded.

No new branch/PR, LLVM change, fresh ordinary full-compressor timing, multi-MB run or parity claim.
