# Prepared record names: measured gain and executable scorecard

Retain the immutable record-name preparation change. On complete level-12
compression of `abcdefgh`, elapsed time falls **0.90895%**:
**4.3892295 → 4.3493335 ms**, or **15.400805× → 15.260819× Roc** with one fresh
shared 0.285 ms reference. All four independent preparations improve, as do
54/64 descriptive blocks. This small positive result is retained.

The separate coherent diagnostic cohort reports **49,006,012.5 → 49,518,676
instructions (+1.046%)** and **13,832,994 → 13,775,388.5 cycles (−0.416%)**.
Compression allocation requests remain **13,994**, requested bytes
**98,586,843**, and additional peak tracked heap **20,575,432 bytes**.
Prepared tracked heap rises **141,595,920 → 142,706,080 bytes** (+1,110,160).
These are explicit time/instruction/memory tradeoffs, not improvements in every
metric. The updated [executable comparison](../executable-comparison/REPORT.md)
shows the reference, previous version, current version and ratios together.
Parity remains outstanding at 1×. No larger-input timing is attempted.

## Revisions and patch

- Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Published start: `5083c1732a15852aa7017d1c9d2685082feba2ef`.
- Frozen baseline compiler: `552e537fcbcfc3811ce442c982010b5e1e999e6c`.
- Implementation: `326576c01541820f3f98839880b90ba4e1e4edb4`.
- Optimized release source: `b8319ff83da250a49e9cee81acb91ba1da5c6a25`.
- Generated-path tests/fixture:
  `9de316926be3c8a5637a12fc145f800e0ccd4315`.
- Stronger finite-prefix oracle assertion:
  `e1f98e261a9d951c2cea1ee1be676bd40e2d9949` (test-only).
- Separate CI inventory adjustment:
  `4b568b3462a115997abcc5abcb3522032f1afa59`.
- Exact new test image/entry audit registration:
  `509382cc1e67dcb04824b4888ee0e0d950ad46e7`.

The final published review head is recorded on PR #973. `analysis-inputs`
archives the complete source/test/workflow patch relative to the published
start, full pre-closure commit inventory, release receipt, library pins,
unchanged input pins and both original primary Object/image bindings.
Typed IR/work/native ABI/bytecode remain **131/95/37/14**. No resource price,
allowance, parser, codec revision or benchmark-source change is made.

## Design and preserved behavior

The real compressor reaches private scalar record fields repeatedly.
`check_buffer_record_ordinal` already receives a resolved field ordinal but
used the maintained name selector to hash immutable schema names on every
visit. `RecordSelector` prepares ordered `(hash, charged byte length)` rows
once through the existing prepared library-call owner. Build-owned image
restoration independently derives the same rows from the checked template;
they are serde-skipped and introduce no wire field or execution authority.

Each invocation still reads its current selector and charges every original
row **before** comparison, in original order. First-match collision behavior,
unknown IDs, partially spent failed prefixes, kind/schema/ordinal checks and
error order stay intact. Preparation captures no current arguments, selected
result, funding, permit or visit. The original maintained selector remains the
fallback when optional preparation refuses. No executor or owner is added.
The host runtime changes; the generated compressor Object remains byte-identical
and its entry text/stack remain **13,885,696 / 108,400 bytes**.

The full compressor source and bytecode SHA-256 remain
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` and
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Pinned mimalloc 2.1.7 Object SHA-256 remains
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
The real tree/cache/path capacity is **262144 U16 / 1500513 U32 / 305257 U64**,
depth 300, nice length 258, with original near-EOF limits and allocation sites.
Every measured Mech call verifies exact ten raw bytes, independent inflate,
output capacity 13 with clear/reuse, input aliases/content, exact work
**60,260,219,686** and zero final managed bytes/nodes.

## Measurement and build evidence

Four matched prepared linked pairs each run 16 alternating-order blocks of
31 calls/version: **3,968 primary calls**, 1,984/version; warm-up/final make
3,984 total. The reference has 20,000 calls. The fixed 0.300 ms historical
ruler also improves **14.630765× → 14.497778×**. Both programs are ready before
primary measurement. No build, test, audit, profile or instrumented diagnostic
overlaps primary. Each reference/pair/summary/counter/profile cohort was
committed immediately, with no live worker restarted after a polling timeout.

The optimized Cargo release completes in **494.280 seconds**, using Rust
nightly 2026-03-03 (`ec818fda3`), LLVM 22.1.0, aarch64, opt 3, warnings as errors,
locked/offline dependencies, incremental off and one build job:

```text
cargo build --release --locked --offline \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --message-format=json --bin native_frame_transport --bin artifact_review \
  --features mech-runtime/compiler
```

Core, Syntax, Serde, SerdeJson and Miniz hashes stay unchanged. Engine and its
Compute/GPU/standard-library dependents rebuild. Full features/profiles,
commands, environment and eight frozen library hashes are in `release-build`.
The hardware remains an M1 Mac mini, 8 GiB, macOS 15.6.1 (24G90).

CPU medians use 32 steady observations/implementation; memory uses eight Mech
and 32 reference observations. Instrumented elapsed is excluded from primary.
No CPU sample incurs a page-in. Boundary-read instructions are not subtracted.
The diagnostic recovery below gives the precise build scope.

Fresh **isolated** profiles use the original primary workers, 695 calls each,
serially after all image emission/counters finish. Current frame-action sample
ancestry remains **268/1048 weights (25.57%)**. Ordinary visits, other invocation
helpers and selected library calls account for 14.12%, 14.60% and 10.31%.
Sampling includes surrounding harness work and is not an exact decomposition of
the compression timer. The first overlapped profile pair is excluded entirely.
Generated Object identity reuses the independently verified static inventory.

## Passing qualification

All commands, feature profiles, stdout/stderr and counts are archived under the
named groups. Maintained unit roots use opt 1 for GPU and opt 0 for Engine, the
recorded release dependency features, and `-D warnings`; component/linked
qualification uses the optimized coherent graph described below.

| Receipt | Selection | Observed result |
| --- | --- | ---: |
| `unit-record-selector` | Engine `record_selector::tests` | **2 passed** |
| `unit-record-generated` | New record lookup and finite-boundary tests | **2 passed**, included in runtime count |
| `unit-runtime` | `native_artifact::program::runtime::`, established stats-import skip | **315 passed** |
| `unit-work-values` | `native_artifact::work::`, `memory::`, `values::` | **31 passed** |
| `qualification-components` | Existing unchanged component filters | **234 passed** |
| `qualification-composition` | `numeric_composition_qualification` | **8 passed** |
| `qualification-path-mechanism` | Real optimal-path nested scalar-child selection | **1 passed** |
| `ci-inventory` | Exact existing CI filters, expanded guard | **17 listed and executed; 132 Python tests passed** |
| `linked-acceptance` | Actual AOT executable, original/decoded, six ordered cases/form | **12 executions passed** |
| `scalar` | Bool/U64/I64/F64 and scalar-loop, original/decoded | **12 controls passed**, original 1.05 ceiling |
| `frame-allocation` | Exclusive/aliased, original/decoded, 0/64 steps | **8 cases passed**, zero allocation slope |
| `architecture` | Existing ten routing/catalog/layout/factory/warning/unsafe/fmt/diff checks | **10 passed** |

There are **591 distinct Rust tests** across the principal six selections;
new tests and CI reruns are not counted twice. The finite-prefix oracle includes
successful, denied and partially spent denied cases, exact failed work receipts,
current/unknown/repeated selectors and first-match duplicate hashes. The frozen
baseline and candidate match **all 124 admission/generated receipts byte-for-byte**.

The maintained generated test proves all four native scalar kinds really reach
private record projection. Original/decoded artifacts and original/restored
same-build images retain changing recursive arguments, expected 36/30/17 outputs,
exact work, ordered assertion/load/arithmetic errors, aliases, prior publication,
cleanup and recovery. Six exact/one-short finite dimensions reuse the maintained
boundary helper. The separate linked executable repeats those ordered cases and
gets the same success work 21,787,354 / 21,862,018 / 21,744,466.
Existing component expected values, allowance ceilings and Huffman work 91,452,918
are unchanged. The CI filter selection is unchanged; its guard rises 15→17 for
the two new tests, separately committed. Architecture checks remain enforced.

## Failures, recovery and skipped coverage

- **Introduced test registration:** the first architecture pass finds the new
  test's unchanged-image/JIT raw boundary missing from the exact allowlist.
  Commit `509382cc1` registers that test path with its existing owner/lifetime
  contract. No production boundary or audit rule is relaxed. All ten checks pass.
- **Measurement setup:** the earlier eight-library freeze omitted the transitive
  Compute artifact. A shared Cargo build replaces/removes that dependency, so
  frozen-GPU counter relinks fail `E0463` before execution. A capture wrapper
  intentionally exits 97, but Cargo can already remove its output first; an
  early assertion of no artifact mutation was too strong. Raw receipts remain.
- Exact-old-dependency attempts include unsupported `--build-plan`, an incompatible
  standalone metadata extraction (`E0786`) and nonmatching crate hashes. They are
  setup failures, not compiler acceptance or performance observations. The final
  shared Compute recompilation permits the maintained GPU unit-root build; it
  does **not** recreate the frozen Cargo GPU's transitive crate hash. A separate
  after-only original-graph CPU relink also fails before execution and is excluded.
- **Diagnostic recovery:** before/after Compute/GPU libraries are explicitly rebuilt
  from the same production sources/features with identical optimized flags,
  original frozen Engine dependencies and coherent search paths. Fresh Object/images
  are emitted from each graph; both Objects are byte-identical to the primary
  Objects. Those matched counter/memory cohorts and optimized component/linked
  checks use this explicitly separate configuration. The original primary workers
  remain unchanged and valid; no mixed image/library authority is assumed. Future
  Cargo work must regenerate its coherent graph rather than copy incomplete freezes.
- The first baseline profile overlaps the last diagnostic image emitter by about
  11 seconds. Both original profile groups are retained but **excluded**. Both
  isolated groups are rerun after terminal completion and alone enter analysis.
- A diagnostic link fails with `errno 28` before execution. Unneeded completed old
  builds/counter payloads are removed; the same cohort then runs and is archived.
  No primary observation is lost or replaced.
- Early witness adapters use an unavailable plan accessor, an invalid F64 literal
  and missing bit wrappers; corrected harness versions match the maintained syntax
  and APIs. Failed commands/diagnostics are archived.
- **Inherited exclusion:**
  `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map` imports `stats`
  unavailable in this established reduced catalog. The preceding round reproduces
  it on the exact clean parent; it remains the sole excluded runtime test. No
  unrelated catalog repair is added.

No unresolved introduced product correctness failure is known. The original-graph
optional diagnostic relink remains unavailable as explicitly recorded; this is
not concealed by the passing coherent rebuild. Full Core/Engine/all-feature suites,
a fresh minimal CI-profile Cargo rebuild, branch/cache misses, total bytes copied
and larger inputs are not newly run/measured. Hosted CI is cancelled by user
direction, not green; PR #973 stays draft and a new review is requested.

`python3 reconstruct.py EMPTY_DIRECTORY` verifies every archived path/length/hash
and regenerates both scorecards byte-for-byte without compiler or benchmark
execution. Observed result: **50 archives, 524 file occurrences, 387 unique paths**.
`scorecard.json`/CSV and `analyze.py` preserve all cohorts and their scopes.
`analysis-inputs`, `diagnostic-final-disposition`, `excluded-initial-profile` and
`architecture-test-registration-failure` retain the unsuccessful commands too.
The next compiler target should follow the measured frame/ownership costs and
repeat this same scorecard, including instruction and memory regressions.
