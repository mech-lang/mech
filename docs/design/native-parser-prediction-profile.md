# Predictive canonical parser

On the Apple M3 Air, predictive dispatch reduces level-12 compressor parsing
from **3.431–3.443 seconds to 2.452–2.494 seconds**, a **28.05% reduction**.
Fresh full source-to-Object compilation takes **50.157–50.558 seconds**, compared
with **51.891–52.454 seconds** for the matched control. The mean phase total
falls 3.48%; this four-observation cohort does not establish a statistical
population or attribute changes in unrelated phases to the parser.

Source change `edd52ccbd53327635f8bfd12d8e2733c6100b87f` uses shared base and
operator specifications to reject impossible alternatives before constructing
temporary markers and checkpoints. The base specifications move the existing
productions out of their executor; both prediction and resumable execution read
the same definitions. Canonical grammar productions and source syntax stay the
same. The retained Unicode scanner continues to determine literal segmentation.

Predictions admit bounded ASCII windows. Whitespace prefixes have a bounded
scan; longer prefixes use the ordinary parser. A complete prediction must fit
the original work allowance, fuel and event reservations. Temporary whitespace
still pays its original token charges, and summaries preserve logical work,
rewinds and peak requirements. Unsealed input, Unicode matching, partial
continuations and tight limits retain their existing execution path.

| Balanced run order | Parse | Frontend | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 3.443 s | 9.014 s | 39.916 s | 52.454 s | 53.512 s |
| Prediction 1 | 2.494 s | 8.326 s | 39.654 s | 50.558 s | 51.631 s |
| Prediction 2 | 2.452 s | 8.308 s | 39.320 s | 50.157 s | 50.400 s |
| Control 2 | 3.431 s | 8.237 s | 40.139 s | 51.891 s | 52.068 s |

Both compilers start from `27a0d7750c9b58833f7c6ce69881d9a094ca4f58` with the
same Rust toolchain and 52 flags. Every run starts a fresh process. No builds,
tests, counters or CPU sampling overlap these timings. Both first executable
launches retain about 0.9 seconds of extra startup outside compiler phase
timers. Compiler bootstrap is separately recorded and excluded. The first
control's longer frontend interval remains in the cohort.

All four source files, revision-14 bytecode outputs, native Objects and complete
metadata files match **byte-for-byte**. The qualified Object remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`.
No runtime implementation or generated instruction changes; there is no new
compression-runtime campaign for this parser change.

Whole-compiler retired instructions fall 2.32% in these observations. Compiler
maximum RSS is an adverse result: control **7,407,878,144–7,600,553,984 bytes**,
candidate **7,763,001,344–7,773,536,256 bytes**. The cause of this difference was
not isolated. Smaller parser work does not establish lower whole-compiler memory.

Separate relaxed-atomic diagnostics confirm the mechanism:

| Physical parser operation | Control | Prediction |
| --- | ---: | ---: |
| Base/operator continuation frame pops | 129,365,126 | 58,758,006 |
| Checkpoint calls | 42,639,485 | 22,723,432 |
| Rewind calls | 35,397,697 | 18,246,758 |
| Mutable journal-page accesses | 13,289,669 | 9,089,605 |
| Literal-cache replay probes | 19,941,589 | 10,798,202 |

These are nested counters, not an additive time partition. Counter elapsed is
unqualified. Public parser statistics stay at 1,967,640 steps, 434,637 emitted
events, 92,316 final nodes and zero recovery bytes for the 160,857-byte source.

The full syntax suite passes **765 tests across 46 suites**, including 523 units.
New controls compare the original executor against predictions for every ASCII
head, whitespace, Unicode neighbors, tight fuel/event budgets and long prefixes.
A separate **1,667-case differential comparison** preserves complete trees,
node/token identities, diagnostics, statistics and restart data exactly. Both
serialized results have SHA-256
`d5ad4ed9d87c64d3295f31eed97d167f6ba5b41ac1a3852b3e00ff1af8e8b2e7`.

The packet retains narrower predictions, empty-result caching and direct
executor trials, setup failures and all observations. One initial compiler
preparation reused the control executable because copied source timestamps
predated its build. Identical binary hashes exposed that before any timing;
touching the changed sources produced the distinct candidate used throughout
the cohort. The invalid preparation remains archived.

The [compact result](../../benchmarks/mech-deflate/native-profile/parser-prediction-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/parser-prediction-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-parser-prediction-apple-m3-20261113.tar.gz):
1,958 files, 11,451,425 compressed bytes, SHA-256
`7200fce6d6c1986371290c0b741ae7e32992c18b138fd0e696176be910033866`.
Independent public retrieval, complete manifest verification and `python3
replay.py` reproduce the compact result byte-for-byte without a compiler.

The full compilation remains about 50 seconds. Frontend work takes about
8.3 seconds and native compilation about 39.5 seconds; the sub-10-second goal
remains open.
