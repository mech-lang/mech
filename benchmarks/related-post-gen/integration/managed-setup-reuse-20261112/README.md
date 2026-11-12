# Managed setup reuse: IR337/338

The selected managed checkpoint remains **IR334**. Neither the static replay-facts cache nor owned worker capture installation established a managed latency gain. The latest selected managed median is **1.439375 ms / 1.250667 ms = 1.150886×** the fastest eligible Rust control. Parity remains unmet.

This packet preserves **1,468 new exact timed/resource outputs**: 1,209 ordinary latency outputs (31 randomized rounds × 39 implementations) and 259 separately instrumented resource outputs (7 × 37). Four new managed adapters also pass the complete 5,000-post input and all 31 maintained edge fixtures. The 96 compact resource-adapter correctness executions were already qualified in the previous cohort; they are retained here as provenance, not counted as new outputs. No independent confirmation was run for the new managed candidates because no useful managed latency gain was established.

## Ordinary latency

The reference is selected prospectively from the 22 eligible complete post-JSON Rust implementations, including two **fresh four-worker** controls. The reference in this cohort is `rust-ordered-mask-tables256-fresh4`. The 20 serial Rust implementations remain visible; two preinitialized controls remain excluded. Every row has 31 exact outputs. Ratios are lower-is-better.

| Implementation | Post-JSON ms | × fastest eligible Rust | Whole process ms |
| --- | ---: | ---: | ---: |
| rust-ordered-mask-tables256 | 1.542500 | 1.233342 | 30.748 |
| host-v2-4 | 1.608500 | 1.286114 | 53.748 |
| rust-ordered-mask-tables256-fresh4 | 1.250667 | 1.000000 | 31.447 |
| retire-overlap | 1.456167 | 1.164312 | 55.364 |
| mech-ir320-compact-native-4 | 1.353458 | 1.082189 | 34.058 |
| scoped-frame | 1.439375 | 1.150886 | 54.458 |
| typed-fixed-byte | 1.533208 | 1.225912 | 61.665 |
| guarded-owner-wide | 1.448959 | 1.158549 | 53.488 |
| guarded-owner-byte | 1.470542 | 1.175806 | 60.148 |
| key-u32-probe | 1.468000 | 1.173774 | 60.010 |
| compact-maintained-wide-4 | 1.301291 | 1.040478 | 34.079 |
| compact-direct-byte-4 | 1.285458 | 1.027818 | 34.175 |
| compact-key-u32-4 | 1.266750 | 1.012860 | 33.298 |
| static-replay-byte | 1.474500 | 1.178971 | 59.982 |
| static-replay-u32 | 1.459584 | 1.167044 | 59.704 |
| owned-capture-wide | 1.444375 | 1.154884 | 54.659 |
| owned-capture-u32 | 1.455333 | 1.163645 | 60.819 |

`first-setup-reuse/config.json`, `protocol.json`, `run.json`, raw per-process outputs and process counters, execution order, and full timing report are retained. Seed: `202611120953`; paired bootstrap seed: `202611120954`. All managed samples launch four fresh tasks in one batch and preserve the exact numeric worker work of their same-compiler full-input qualification. Total work also matches exactly for every managed implementation except the historical, unselected IR335 `typed-fixed-byte` control: its qualification total is 36,997,602,022, while its retained primary totals range from 36,997,599,934 to 36,997,603,102. The cause is **unattributed** in this packet; its numeric worker bill stays exactly 24,547,911,858. All per-variant total-work histograms and qualification comparators are retained, and no total-work constancy is claimed for that one control. The exact total-work checks remain strict for all other implementations. Three scheduled observations have lower simultaneous concurrency: selected IR334 round 25 has peak 2; IR336 wide round 26 has peak 2; IR337 byte round 26 has peak 3. All other managed observations have peak 4. The complete per-variant histograms are in `report.json`; every observation remains in the timings. `verifier-development.json` records correction of the initial copied verifier's overly strict peak-4 assertion; no executable was rerun and no sample was filtered.

Paired changes use the median of 31 per-round percentages with 10,000 bootstrap draws and indices 250/9749:

| Candidate / control | Paired change | 95% interval | Faster pairs |
| --- | ---: | ---: | ---: |
| IR337 byte / IR336 byte | +0.8434% | [+0.2968%, +1.6138%] | 9/31 |
| IR337 U32 / IR336 U32 | −0.7724% | [−2.0502%, +0.0657%] | 20/31 |
| IR338 U32 / IR337 U32 | +0.1541% | [−0.7917%, +1.1112%] | 14/31 |
| IR338 wide / IR336 wide | −0.3649% | [−1.1971%, +1.6032%] | 17/31 |
| IR338 wide / selected IR334 | +0.4951% | [−1.1023%, +1.3995%] | 14/31 |
| IR338 U32 / selected IR334 | +1.4148% | [−0.1751%, +2.9275%] | 11/31 |

IR337 byte regresses; the other new managed comparisons remain inconclusive. Current IR336 compact U32 is visible at 1.012860× Rust in this cohort, under its separate compact/unmetered contract. This cohort is not a new confirmation or parity claim for compact mode.

The authoritative calculation clock is one continuous post-JSON interval through input preparation/admission, native execution, fresh worker creation/run/join, internal cleanup and result materialization. Artifact restoration, JSON IO, serialization and final decoded-input/result destruction remain visible in whole-process time. Whole-process performance is **not an acceptance gate**. The full adapter-specific boundaries remain in the retained configuration and resource plan.

## Separate resource cohort

These are process-wide/all-thread hardware and tracked allocator medians for the continuous post-JSON interval, not measurements used to choose latency winners. Seed: `202611120955`. Whole-process resource medians, per-phase counters and executable/native code/stack sizes remain in the complete resource report and plan.

| Implementation | Instructions | Cycles | Allocation requests | Requested bytes | Additional live heap peak bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| rust-ordered-mask-tables256-fresh4 | 18,167,089 | 6,591,534 | 78 | 7,026,975 | 5,721,694 |
| rust-ordered-mask-tables256-fresh4-apple-m3 | 18,142,454 | 6,612,828 | 78 | 7,026,975 | 5,721,694 |
| host-v2-4 | 31,485,264 | 9,861,021 | 735 | 7,036,985 | 4,871,859 |
| scoped-frame | 29,687,650 | 8,945,948 | 725 | 6,376,193 | 4,211,067 |
| guarded-owner-wide | 29,689,429 | 8,972,003 | 725 | 6,376,209 | 4,211,067 |
| guarded-owner-byte | 29,798,397 | 9,146,408 | 725 | 6,404,707 | 4,397,235 |
| key-u32-probe | 29,560,437 | 9,319,411 | 725 | 5,502,843 | 3,495,371 |
| static-replay-byte | 29,740,839 | 9,106,667 | 725 | 6,404,707 | 4,397,235 |
| static-replay-u32 | 29,503,682 | 9,184,159 | 725 | 5,502,843 | 3,495,371 |
| owned-capture-wide | 29,673,840 | 9,059,714 | 725 | 6,376,209 | 4,211,067 |
| owned-capture-u32 | 29,525,270 | 9,141,890 | 725 | 5,502,843 | 3,495,371 |
| compact-maintained-wide-4 | 30,028,370 | 7,713,545 | 71 | 3,926,240 | 2,997,872 |
| compact-direct-byte-4 | 30,071,136 | 7,680,791 | 71 | 3,768,474 | 2,997,872 |
| compact-key-u32-4 | 30,149,801 | 7,613,821 | 71 | 2,855,586 | 2,084,984 |

The new managed variants do not reduce allocation requests, requested allocation bytes or additional live heap peaks versus their matching IR336 source controls. Small instruction changes do not establish latency gains. The IR337 cache occupies existing compiled descriptor/proof metadata; those additional compiled metadata bytes are **not claimed to be funded by the invocation's managed storage budget**.

Tracked Rust heap requests exclude allocator internals, OS stacks and native `mmap`. Existing compact native payload request/byte/peak statistics are retained separately in every raw adapter record; they are not added blindly to Rust heap or OS footprint values. RSS and allocator peaks are different measurements. Free-call counts were not instrumented; freed bytes are retained. The helper and every adapter/control/bundle/library pin are preserved. Ordinary and instrumented executable sizes remain separate; compact metadata regions may share code and are not summed or treated as equivalent to the managed root frame.

## Compiler changes and source closure

Production backend: **Cranelift**. Work policy 127, ABI 42, image 11 remain unchanged.

**IR337** derives the two checked static typed-lookup replay allowances once from final immutable descriptor metadata. Unsupported calls, mismatches and overflow leave no cache. Every restored image rederives the private serde-skipped facts, including Object-bound restoration. The existing dynamic source, owner, generation, retirement, payload, search/sort and physical-attempt checks remain in their original runtime order. The selected wide IR334 workload does not use the narrowed replay scan. No invocation cache copy or extra per-call allocation was introduced.

**IR338** moves an already owned `Capture::Value` into a proved fresh worker slot. A pure preflight establishes the exact arena/work identity, type/schema, owner, empty generation-zero slot and unchanged account limit. The same two original individual debits (32, then retained-size × 4 + 32) execute in order under the existing bounded engine debit batch. A miss keeps the ordinary insertion path; denial keeps the input owner alive until the batch/account/TLS guards have released. No new ledger, aggregate prepayment, host scoring path or compatibility bridge was added. The source count predicts 32 fewer successful escrow acquisitions over four selected workers; that is a source-level estimate, not an instrumented lock count or speed claim.

The exact 13-file overlay over published IR336 `4b6133f85ae85f858bfe29f8cc871aada58a77bb` is retained as individual content-addressed source records plus `measured-source/IR336-to-IR338.patch`. Complete inventories cover 2,816 files for IR337 and 2,817 for IR338 (three new test files over the 2,814-file base), with 164 literal includes checked against the base. Both coherent 12-crate build closures are retained, including unchanged non-GPU stage command/environment/source/library identities and exact adapter library/descriptor/Object bindings. The root independently audited and committed the exact IR338 overlay as **`6f7e0458e088cf7e50c97ebfe2d15e7bca02b22f`**; `IR338-publication` retains that receipt. After the docs-only rebase onto #973 `8f743bd3977fa2cfa7025868c599c775d002b870`, the source-equivalent alias is **`b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164`**. `IR338-rebase-latest973/receipt.json` records all 2,817 source files unchanged and zero production changes in the new base. The verifier requires every inventory entry to match the published source alias and IR336 alias `0fca4d1030556dda112da254cb09e2ea5fcb7577`. It also checks the original pre-rebase commits when those historical objects exist locally; they are not needed in a fresh checkout.

The maintained Mech source remains SHA-256 `d70a001620977aff9529884d0207fbf7952a359388988431b4102ae0465f1905`. The direct U8-ID and U32-key sources are the previously qualified IR336 source variants. Ordinary and resource executables use the exact same native Object and compiler closure; same-Object audits retain exact function addresses and text after separately verified relocation fields. Executables and libraries are represented by build/link/hash receipts rather than duplicated large binaries.

## Contracts and verification

The combined IR338 immutable test executable passes **50 focused tests plus one distinct compact typed test**: replay cache equivalence/overflow/restore, typed finite replay and exact paid/error prefixes, owned worker capture denials/ownership, worker cold/retry/alias/restore, guarded dictionaries, unchanged Huffman bills and work meter contracts. The complete native suite has **1,079 passes, 16 failures, 11 ignored and 35 filtered**. All 16 failure names are identical to the retained IR334 baseline; there are no new broad-suite failures. Early scratch audits retain their original “not run” wording and are superseded by later explicit build/test receipts rather than rewritten.

The content-addressed `records.json` manifest maps every logical evidence path to a gzip blob with compressed and uncompressed hashes. Identical bytes are stored once within the packet. Source inventories are complete; only changed bytes are copied. The previous `typed-native-storage-20261112` packet remains unchanged.

Run from the repository checkout:

```sh
python3 benchmarks/related-post-gen/integration/managed-setup-reuse-20261112/verify.py
```

The verifier performs **no native executions**. It checks the manifest, all exact outputs, randomized orders, ordinary timer parsing and medians, paired bootstrap intervals, resource raw records and medians, source inventories/overlay/commit, unchanged literal includes and build closure, same-Object and edge qualifications, focused test counts and broad failure attribution. `verification.json` preserves the packet author's original receipt. `verifier-commit-availability.json` records the later source-alias portability correction; `root-verification.json` records the final root check.
