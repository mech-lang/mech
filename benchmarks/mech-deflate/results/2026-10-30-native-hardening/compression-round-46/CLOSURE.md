# Exact funded record-admission prefixes — retained

Complete eight-byte level-12 compression improves **1.98944%**: **4.2935205 → 4.2081035 ms**, or **14.856472× → 14.560912× Roc**, using one shared fresh **0.289 ms** reference. All four independently prepared pairs and **60/64 descriptive blocks** improve. There are **3,968 measured calls**, 1,984/version, plus separately marked warmup/profile calls. Do not multiply independent earlier cohorts. Parity remains outstanding.

| Per compression | Before | After | Change |
| --- | ---: | ---: | ---: |
| Instructions | 48,599,979 | 46,993,498.5 | **−3.30552%** |
| Cycles | 13,551,678 | 13,282,488.5 | **−1.98639%** |
| Allocation requests | 13,994 | 13,994 | unchanged |
| Requested bytes | 98,586,843 | 98,586,843 | unchanged |
| Additional peak tracked heap | 20,575,432 | 20,575,432 | unchanged |
| Prepared tracked heap | 142,706,080 | 142,829,248 | **+123,168 bytes** |
| Resident memory after compression | 319,668,224 | 319,799,296 | +0.0410% |
| Kernel physical footprint after compression | 228,640,512 | 228,771,584 | +0.0573% |

Timing, hardware, allocator and sampling cohorts are separate and serialized. CPU counters include uncorrected boundary/instrumentation overhead. RSS, physical footprint and tracked heap are different observations and must not be added. Prepared state includes the compiler/catalog and descriptor; this is not a minimal-runtime RAM figure. Full reference ratios and the fresh hot-path profile are in [PROFILE.md](PROFILE.md).

## Exact revisions and mechanism

- Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Published baseline: `b12f6c5afb2c859f6cde29ba566ddf09e70d2f79`.
- Implementation: `fead50f19bcec08deacc4622c4ded05ee91b8383`.
- Optimized Engine/host source: `6027e8016150f0510b23fcdda20430a40837e1da` (subsequent changes are receipts/documentation).
- IR/work/native ABI/bytecode: **131/95/37/14**, unchanged. Final review head is recorded on PR #973.

Round 42 already retained field IDs and name-byte costs; round 43 already moved disposable pack handles. This patch prepares each original record-lookup prefix's exact compute/comparison totals and first matching ID. The current selector must still match the expected first ordinal. Only if the entire projection demand and the same active account's limits fit does it pay that exact successful prefix once. It does not cache permission, arguments or previous results.

The scoped debit uses the existing checked-local-account function, including reserved work and enclosing relative caps. It leaves observer high-water marks unchanged, just like the original direct charges. A declined preview spends nothing and runs the original ordered per-name path. Foreign/absent scopes, shared worker credit, changed/unknown/colliding selectors, overflow and insufficient funding retain that path. No worker-credit preview/debit race is introduced.

The earlier same-Object control census identified 741 projections and 3,581 per-name visits: an opportunity for 2,840 fewer transactions. These are retained graph-derived counts, not a new branch-counter measurement. The directly observed improvement is the matched timing and **1,606,480.5 fewer hardware instructions**. Targeted real-binary disassembly preserves the new prefix helper, its scoped debit, the ordered fallback and the selected caller. The smaller caller alone is not a total static-code saving: a new helper is emitted.

The generated compressor Object remains byte-identical, SHA-256 `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`; generated text/stack remain **13,885,696 / 108,400 bytes**. `generated-artifact-once` now archives the actual **13,886,424-byte Object**, **59,784,245-byte image**, metadata and binding, compressed together once to about **5.25 MB**. Future instruction inspection need not rely on counts alone.

## Configuration, validation and failures

The selected-operation Cargo release command is unchanged apart from verbose logging: locked/offline, one job, opt 3, warnings-as-errors, `mech-runtime/compiler`, native-profile manifest. It passed in **498.535 s**. The host is then built with the exact retained direct-rustc flags/features against the newly frozen matching Engine, Stdlib and Compute artifacts (**22.488 s**). Both primary variants and their CPU/memory consumers use their respective identical coherent library graphs and same-build Objects/images. No dependency is swapped under a running consumer.

The recorded producer is nightly **2026-03-03**, LLVM 22.1.0, aarch64, Apple M1. GPU features remain `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`; test optimization/configuration is recorded per command. Existing source algorithms, tables, search limits, buffers, allocation sites, pinned mimalloc 2.1.7, 13-byte output clear/reuse and evaluate_into timer are unchanged. Input is fully loaded before timing; no streaming or larger-input throughput claim is made.

All primary calls pass exact raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, input aliases/content, exact work **60,260,219,686**, capacity 13 and final zero cleanup. A separate same-library diagnostic on both versions also verifies exact comparison work **47,612**, with no change to primary observations.

| Validation | Observed result |
| --- | --- |
| Record-selector regressions | **5 pass**, including **10,560** exact-result/error/spent-prefix differential cases across four independent limit authorities |
| Existing budget tests | **26 pass** |
| Worker/credit/debit tests | **36 pass** |
| Maintained native runtime tests | **316 pass**, one inherited exclusion below |
| Native work/memory/value tests | **31 pass** |
| Existing component qualification | **234 pass**, unchanged goldens/allowances |
| Connected qualification + path mechanism | **8 + 1 pass** |
| Distinct Rust tests | **657 pass** |
| Actual linked original/decoded executions | **12 pass**, current arguments, assertion/load/arithmetic precedence, prior publication, aliases, cleanup and retry |
| Scalar performance controls | **12 pass**, all existing ≤1.05 ratio and allocation gates preserved |
| Frame allocation controls | **8 pass**, no request/byte/work increase, zero loop allocation slope |
| Existing CI inventory | **18 listed / 18 executed**, guard unchanged |
| Workflow/inventory Python | **132 pass** |
| Architecture/catalog/format checks | **10 pass** |

No unresolved introduced compiler/test failure remains. The sole inherited skip is `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`, whose stats import is outside the reduced linked catalog. No broad all-features or hosted-CI pass is claimed; hosted CI is cancelled by the user's standing instruction and PR #973 remains draft pending green CI and clean review.

Two non-product interruptions are preserved: (1) the allocation driver initially applied a steady-state count to its explicitly marked cold warmup; measured samples already matched, and correcting the filter changed no allowances or compiler behavior; (2) the temporary checkout lost `.git` and 2,325 tracked files after successful component compilation. Common metadata and head were intact. Missing files alone were restored from that exact head, existing files were preserved, and validation resumed successfully. Commands, errors and recovery inventory are archived.

Rounds 44/45 remain separately preserved and withdrawn: mixed elapsed for the borrowed selector; an unmeasured refinement's module/import compile errors. Their source/workflow changes were restored to the qualified baseline before this experiment; pack transfer and prepared name IDs remain retained.

## Reproduction and next bounded step

Run `python3 reconstruct.py EMPTY_SCRATCH_DIRECTORY`. It verifies each archive/file hash and reproduces JSON/CSV scorecards, small-allocation attribution and numerical profiles byte-for-byte. Full commands, features, source patch, build pins, observations and failure dispositions are included. The [allocation report](ALLOCATIONS.md) reconciles all 13,944 small requests across two fresh traces with source constructors and lifetimes.

The latest supplied brief makes **unique-root first admission** the next isolated pilot: count actual eligible roots, reuse the consumed exclusive unadmitted root at the original commit point, preserve shared/imported paths, then repeat the same small benchmark and fresh numerical profiles. Physical cleanup-domain proof is the parallel compiler investigation. Scalar footprint reuse, virtual managed projections, physical storage layout, shared native bodies and numeric-window facts remain distinct later experiments. No combined unmeasured speedup is attributed to them.
