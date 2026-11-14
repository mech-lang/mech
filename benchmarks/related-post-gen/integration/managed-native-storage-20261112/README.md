# Managed native storage and short UTF-8 keys: historical IR331 evidence

This packet records the completed IR331 experiment at compiler source commit
`708511df16e803b5947e84574ba61e3f6d994737`. It is a historical result, separate from
later compiler experiments. The production backend is Cranelift; this work adds
no mandatory dependency.

The branch was subsequently rebased onto PR973 base
`67d7562bc4efda7b3ec8cacb0c346c4e15f4e2e5`. Its published equivalent source is
[`f361f91f298b87ebb9bee880930951fdf79b0d5f`](https://github.com/mech-lang/mech/commit/f361f91f298b87ebb9bee880930951fdf79b0d5f).
All 2,813 measured production source entries are byte-identical at that commit.
The intervening upstream changes are documentation and the DEFLATE experiment
ledger, with no production source changes. Original measured commit identities
remain in raw receipts; `rebase-latest973/after.json` and
`upstream-compiler-audit-latest.json` preserve the equivalence audit. Offline
verification uses the published commit so a fresh clone need not contain the
old, rebased Git object.

The selected managed implementation takes **1.450125 ms**, or **1.195× the
fastest eligible Rust** in its 31-round primary cohort. It beats the fastest of
the original twenty serial Rust controls at **0.960×**, but the newly qualified
Rust implementation with four fresh workers is faster. The target remains
unmet. “× Rust” is Mech elapsed time divided by Rust elapsed time; lower is
better. “Fastest” means fastest among the retained, eligible measurements here,
not a claim about every possible implementation.

## Results and measurement boundary

All latency rows below use ordinary, uninstrumented separate processes. The
primary measure starts after JSON decoding and includes preparation, worker
creation/run/join, and result materialization. Two retained Rust controls that
prepare resources before their processing timer are ineligible as references.
Restore and JSON decoding have separate resource windows. Historical final
input/output destruction remains outside the ordinary post-JSON timer; the
separate whole-process measurement covers launch through exit and is secondary,
not an acceptance gate.

| Primary, 31 rounds | Post-JSON ms | × current fastest Rust | × original serial Rust |
| --- | ---: | ---: | ---: |
| Managed host-v2 control | 1.616166 | 1.332 | 1.070 |
| Managed short-overlap | 1.485542 | 1.224 | 0.983 |
| Managed arena-overlap | 1.477000 | 1.217 | 0.978 |
| Managed retire-overlap, selected | **1.450125** | **1.195** | **0.960** |
| Fresh four-worker Rust, default target | 1.213250 | 1.000 | 0.803 |
| Original fastest serial Rust | 1.510667 | 1.245 | 1.000 |

The primary selected whole-process median is 46.883 ms; the host control is
47.945 ms. These include restoration and JSON I/O and must not be interpreted
as the post-JSON calculation time.

Independent orchestration then ran thirteen randomized rounds over all
twenty-four Rust controls and the host, short-overlap, and selected managed
implementations: **351 exact outputs**. It uses the same machine and input,
with its own implementation of launching, timing extraction, validation,
pinning and aggregation; it does not call `compare-implementations.py`.

| Independent confirmation | Post-JSON ms | × current fastest Rust | × original serial Rust |
| --- | ---: | ---: | ---: |
| Managed host-v2 control | 1.558292 | 1.277 | 1.030 |
| Managed short-overlap | 1.430542 | 1.172 | 0.946 |
| Managed retire-overlap, selected | **1.415958** | **1.160** | **0.936** |

The confirmation supports the broad improvement over the host control: paired
median −8.527%, bootstrap 95% interval [−9.615%, −7.936%], thirteen of thirteen
pairs faster. It **does not establish an incremental storage latency gain over
short-overlap**: paired median −0.972%, interval [−2.075%, +1.445%], eight of
thirteen pairs faster. The earlier primary incremental estimate was −2.045%,
interval [−3.199%, −1.622%]. The confirmation interval spans zero, so this
packet makes no established “2% storage latency improvement” claim. Ratios of
medians in the tables differ from medians of paired ratios by construction.

## What changed

The generated UTF-8 histogram stores an exact packed representation of strings
up to eight bytes. Equal length and equal packed bytes prove short-string
equality, avoiding another byte comparison. The prepaid/unmetered native path
uses bounded, overlapping head/tail little-endian reads; it assumes neither
padding nor ASCII. Longer strings retain their maintained chunked hash and full
equality comparison. The finite metered path retains exact chunk debit and
refusal behavior.

Checked worker inventories omit proven-unused physical categories and shorten
captured-value prefixes. Core fixed arenas initialize the required physical
prefix and retain actual storage bounds while preserving full logical
admission. Workers borrow the immutable, checked retirement table from their
descriptor instead of cloning it for each worker. These are compiler/runtime
storage improvements; the benchmark algorithm is unchanged. Admission,
arithmetic/index checks, handle validation, comparison/work charging, failure
ordering, and retry remain enabled.

The selected benchmark source is `mech/related_optimized.mec`, SHA-256
`d70a001620977aff9529884d0207fbf7952a359388988431b4102ae0465f1905`.
The selected adapter is `hosts/managed-4.rs`, SHA-256
`484c5a1794faa7d3f1834078230e1448fc87552def6baf31cda09e9522d8f78c`.
The generated native object is
`4afa05274aa9fd6197319b838c946e737858dc1dbd25002fa329377fd969c079`.
Each candidate has its own retained build and native-link audit; objects differ
between revisions. Code-size equality does not imply object identity.

## Resources

Seven separately randomized resource rounds cover twenty-nine implementations
and produce **203 exact outputs**. Instrumented timings do not select a latency
winner. The figures below are medians of **all-thread complete post-JSON
windows**. Requests include allocator allocations and reallocations. Peak heap
is additional live requested heap above the window's initial live amount;
it excludes allocator internals, mappings, and OS stacks. Freed bytes are
available; the number of free calls is not recorded.

| Implementation | Requests | Requested bytes | Additional peak heap bytes | Instructions | Cycles |
| --- | ---: | ---: | ---: | ---: | ---: |
| Managed host-v2 | 735 | 7,036,985 | 4,871,859 | 31,485,767 | 10,071,488 |
| Managed short-overlap | 735 | 7,036,985 | 4,871,859 | 30,284,577 | 9,439,734 |
| Managed arena-overlap | 725 | 6,501,273 | 4,336,147 | 30,036,846 | 9,399,487 |
| Managed retire-overlap | **725** | **6,376,193** | **4,211,067** | **29,938,779** | **9,169,639** |
| Fresh four-worker Rust, default | 78 | 7,026,975 | 5,721,694 | 18,178,565 | 6,655,032 |

All allocator accounting underflows are zero. Selected freed bytes are
5,654,209; frees can include allocations made before the window. Compared with
short-overlap, the selected storage changes remove ten requests and 660,792
requested/peak additional heap bytes, with the same input and exact output.

Whole-process peak RSS is recorded separately: selected 26,361,856 bytes,
host 27,115,520 bytes, and fresh four-worker Rust 6,193,152 bytes. These are not
post-JSON heap peaks or the endpoint resident/footprint readings in each
measurement window. `/usr/bin/time -lp` counters, restore, JSON, endpoint
resident/footprint counters and their scopes are retained in full.

Selected checked metadata reports 1,021,740 native code bytes and a 1,568-byte
root stack frame. Other native function stack frames are separately retained;
this is not the process's total stack allocation. Selected ordinary executable
size is 17,108,464 bytes, instrumented executable 17,221,296 bytes, and checked
native metadata 4,640,678 bytes. Fresh default Rust executable sizes are 435,520
ordinary and 485,296 instrumented bytes. Mach-O section/segment inventories use
a Python parser; they are not LLVM-size BSD text totals or resident memory.

## Correctness, tests and diagnostic limits

The three primary cohorts retain 868, 961 and 868 exact full outputs; with
confirmation and resources that is **3,251 timed exact outputs**. Every output
was checked against the complete 5,000-post Gold JSON, with full stdout,
stderr, exit status, order, timing and input/source/executable pins retained.
The final candidate also passes full 5,000-post qualification and all thirty-one
edge cases.

IR331 focused GPU tests: **129 passed, one ignored**. Core tests: **399 passed**.
The broader native run belongs to **IR326**, with 1,051 passed, sixteen failed,
eleven ignored. All sixteen named failures match and reproduce in the unchanged
baseline; the attribution and original receipts are retained. This is not a
claim of a broad IR331 green run or hosted CI success. The focused receipt
preserves an initial selector that found zero tests and the corrected image
selection that passed twenty-seven tests.

The retained ten-second profile is **IR328 diagnostic evidence**, not a final
IR331 profile. Its repeated full path includes JSON I/O, preparation and final
destruction as labeled. Leaf sample counts are not wall-time shares, and warm
diagnostic timings are not qualification measurements.

Both primary and confirmation use the same Apple M3 machine. Independent
orchestration is not independent-machine replication. Five historical
instrumented Rust source/receipt pairs were unavailable locally; their
published provenance hashes and unchanged executable identities remain
explicitly labeled in the resource plan. The fresh four-worker default and M3
controls retain source, Cargo manifests/locks, instrumented source, tests and
qualification receipts.

## Packet layout and offline verification

`report.json` contains derived results and explicit scopes. `records.json`
maps historical logical paths to gzip blobs under `records/`, with compressed
and uncompressed SHA-256 hashes and sizes. Repeated Gold/output/source bytes
share blobs. Build closures and inventories pin sources, binaries and libraries;
executable and library binaries themselves are not included. Generated native
objects and bundles are retained as evidence data. Frozen source snapshots and
the production source inventory preserve revision identity. All 2,813 inventory
entries match the pinned production commit.

The packet contains 13,035 logical records in 3,467 unique blobs, totaling
18,639,730 compressed blob bytes. `assemble.py` documents the original absolute
artifact roots and assembles only historical evidence; it refuses to overwrite
an existing packet. It never launches a compiler or benchmark. The live
experimental compiler is not an input.

From a repository checkout containing the published equivalent source commit, run:

```sh
python3 benchmarks/related-post-gen/integration/managed-native-storage-20261112/verify.py
```

`verify.py` runs no compiler or benchmark. It validates every blob, all complete
outputs and timing/order receipts, recomputes medians/references and paired
bootstrap intervals, checks complete-window resource counters, verifies
qualification scope, and compares the production inventory with Git. Its saved
successful output is `verification.json`.

To inspect one retained logical record without rebuilding:

```python
from pathlib import Path
import gzip, json
packet = Path('benchmarks/related-post-gen/integration/managed-native-storage-20261112')
index = json.loads((packet / 'records.json').read_text())
entry = next(row for row in index['records']
             if row['source'] == 'first-arena-retire/paired-comparison.json')
print(gzip.decompress((packet / entry['path']).read_bytes()).decode())
```
