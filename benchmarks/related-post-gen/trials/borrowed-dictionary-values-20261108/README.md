# IR306: borrowed dictionary keys and independent numeric values

IR305 remains selected for processing. IR306 is a qualified resource experiment, with no confirmed latency gain. Cranelift remains the production backend; no LLVM dependency was added.

| Same-cohort version | First ms | × fastest Rust | Confirmation ms | × fastest Rust |
|---|---:|---:|---:|---:|
| mech-ir305-native-dictionary-seed-frames-4 | 1.785375 | 1.149784× | 1.782541 | 1.150522× |
| mech-ir306-owned-dictionary-values-control-4 | 1.785834 | 1.150079× | 1.786708 | 1.153211× |
| mech-ir306-borrowed-dictionary-values-4 | 1.798084 | 1.157968× | 1.789417 | 1.154960× |

Fastest eligible Rust is 1.552792 ms first and 1.549333 ms in confirmation, both rust-ordered-mask-tables256. All 22 Rust controls remain; 20 qualify for matched processing. The historical selected IR305 checkpoint was 1.768708 ms / 1.151190×; use the same-cohort comparisons above to assess this experiment.

Candidate vs selected IR305 paired processing:
- First: +0.9739% (95% [+0.2481%, +1.7199%]), 9/31 favorable pairs.
- Confirmation: +0.3509% (95% [-0.0188%, +1.1108%]), 11/31 favorable pairs.

All 2,046 latency and 476 resource samples match the Rust oracle. Two prospectively seeded randomized cohorts retain every first/adverse observation. Processing is a continuous post-JSON interval including preparation, allocation, native computation, worker create/run/join, cleanup and result materialization. Artifact restoration and JSON IO remain in complete-process time. Whole-process regression is recorded and is not an acceptance gate.

## Resource measurements, confirmation

| Parameter | Selected IR305 | Borrowed bank IR306 |
|---|---:|---:|
| allocation_requests | 749 | 720 |
| requested_allocation_bytes | 7,727,508 | 7,726,381 |
| peak_live_requested_heap_bytes | 14,105,391 | 14,105,609 |
| peak_additional_live_requested_heap_bytes | 5,760,767 | 5,760,799 |
| instructions_retired | 33,079,622 | 32,978,860 |
| cycles | 10,423,023 | 10,450,290 |
| native code bytes | 1,031,168 | 1,031,168 |
| maximum generated stack bytes | 1,600 | 1,600 |
| executable bytes | 17,127,264 | 17,147,792 |
| map owner slot bytes | 968 | 976 |

Allocation requests fall 749→720 and requested bytes fall 1,127 B. Peak additional requested heap grows 32 B. Instructions fall 91,201 in the first cohort and 100,762 in confirmation. Cycles improve initially and regress slightly in confirmation. These are separate resource observations, not a speed claim.

Complete process confirmation: 45.874375→48.416417 ms. Restoration: 15.755125→18.784875 ms. Extra proof rederivation is a candidate explanation, not a measured causal attribution.

## Compiler change and qualification

A complete-use proof selects a fixed-key dictionary whose only writer takes an admitted borrowed key ordinal and whose completed result feeds native UTF8 lookup. A genuine immutable Map snapshot pins keys. Separate current U64 cells preserve seed values even when another equal key set has different histogram counts. Updates use the sealed ordinal and stable key identity without owned key copies or hashing. The generated source CFG still computes all values. Generic ownership/admission helpers remain native Rust runtime code; this pass does not eliminate their call boundaries.

Different key sets retain ordinary storage. Public escapes keep canonical publication. Original factory/schema/account checks, current generations, finite work/output/clone/temp/node limits, first errors, output atomicity, cleanup and remaining fallback allowances are preserved. The compact bank uses a fallible separately funded box; exact and one-short transient allocation boundaries are tested. No new unsafe storage code or special benchmark kernel was introduced.

Current qualification: 474 native CI tests, preserving all 466 parent names; 176 Map tests (all 173 parent names); 29 image/caller tests; 28 UTF8/range/protocol tests. All 138 Engine collection and 37 projection/frame names pass in 168 unique tests. The 128 scalar and 5 DEFLATE records match every non-timing field, including allocations and work. Each linked AOT variant passes 32 correctness cases. Full original/restored 5,000-post runs confirm one borrowed value bank, zero owned-key imports, one native seed, zero canonical seed tuples, 28 borrowed cells, no canonical String cells, and one completed native dictionary lookup.

All first failures are retained. The discarded inline bank increased owner slots to 1,160 B; the final compact bank is 976 B vs 968 B before. A first expected-size estimate of 984 was corrected to the measured 976; the Option-field-sum invariant remains exact. The O4→O5 production build difference is only that cfg(test) expectation correction; scalar/DEFLATE controls run O4 and final original/restored/CI checks run the exact O5 source. Completed unused local development artifacts are losslessly compressed with SHA/mode manifests; selected comparison runners remain intact.

## Fresh profiling

A new sample observes 10,001 fresh native invocations of the production Object and verifies the exact final output. Inclusive sample stacks overlap and are not CPU percentages. A separate 64-output capture keeps production Engine and Objects unchanged, using eight randomized repetitions, coarse/detail probes and ordinary controls. All outputs and coarse boundaries/counters balance. Diagnostic times include probes and phase medians are not additive production savings.

| Coarse phase | IR306 µs | Owned control µs | Rust µs |
|---|---:|---:|---:|
| preparation_interning_and_eligibility | 739.770 | 746.042 | 609.021 |
| triple_index_and_coverage | 353.125 | 351.479 | 186.250 |
| quartet_index_and_masks | 268.604 | 268.687 | 198.688 |
| queries_cleanup_and_materialization | 486.792 | 482.291 | 575.854 |

Detailed import is 37→8 requests and 20.979→18.188 µs. Key reads and writes remain 56 helper callbacks, about 28.043 and 29.480 µs in detailed captures. The storage change removes allocation and instructions but leaves common per-call admission/caller/bridge work. These callback medians do not establish an isolated total-program speed effect.

Next compiler work should lower the fixed-key ordinal read/write loop through a native loan while preserving source ordering, finite per-call obligations and error/fallback behavior. Larger aligned gaps remain serial triple and quartet construction. The separately preserved completed-element-range proof needs a prefix-extent relation before consumer scatter checks can be removed. Histogram canonical publication still costs 172 allocations. Keep this bank as a foundation and combine it with lowering of calls, rather than promoting its present latency.

## All retained Rust timings, confirmation

| Rust version | Reported processing ms | × fastest eligible Rust | Matched processing eligible |
|---|---:|---:|---|
| rust | 8.826709 | 5.697103× | yes |
| rust-preinitialized | 1.927458 | 1.244057× | no: preinitialized pool |
| rust-mech-control-streaming | 1.642750 | 1.060295× | yes |
| rust-mech-control-u32-casts | 1.646042 | 1.062420× | yes |
| rust-mech-control-u8-ids-u32 | 1.641167 | 1.059273× | yes |
| rust-ordered-scan-only | 1.597333 | 1.030981× | yes |
| rust-ordered-ordered | 1.577417 | 1.018127× | yes |
| rust-ordered-mask-reuse | 1.583583 | 1.022106× | yes |
| rust-ordered-ordered-mask | 1.573375 | 1.015518× | yes |
| rust-ordered-ordered-mask-tables | 1.566167 | 1.010865× | yes |
| rust-apple-m3 | 8.819792 | 5.692638× | yes |
| rust-preinitialized-apple-m3 | 1.863750 | 1.202937× | no: preinitialized pool |
| rust-mech-control-streaming-apple-m3 | 1.652541 | 1.066614× | yes |
| rust-mech-control-u32-casts-apple-m3 | 1.644500 | 1.061424× | yes |
| rust-mech-control-u8-ids-u32-apple-m3 | 1.657959 | 1.070111× | yes |
| rust-ordered-scan-only-apple-m3 | 1.650250 | 1.065136× | yes |
| rust-ordered-ordered-apple-m3 | 1.646875 | 1.062957× | yes |
| rust-ordered-mask-reuse-apple-m3 | 1.651250 | 1.065781× | yes |
| rust-ordered-ordered-mask-apple-m3 | 1.649500 | 1.064652× | yes |
| rust-ordered-ordered-mask-tables-apple-m3 | 1.651042 | 1.065647× | yes |
| rust-ordered-mask-tables256 | 1.549333 | 1.000000× | yes |
| rust-ordered-mask-tables256-apple-m3 | 1.553750 | 1.002851× | yes |

Source commit: [34afe038](https://github.com/mech-lang/mech/commit/34afe038f933e3a2dff96a42b591f2617bd8fe2a), branch `codex/related-borrowed-dictionary-values-20261108`. PR #990 keeps its separate integrated IR242 head and targets the PR #973 branch; this measured research is published separately.

Measurements retain all schedules, complete resource fields and exact output aliases. Builds and tools pin source/dependency/flag/link recipes. Source copies end in .rs.txt so evidence does not change compiler inventories. Output aliases contain SHA/byte count and shared byte-for-byte blobs. Retained library archives have verified lossless readback; restore from the recorded gzip path/mode before recreating old recipes. Use the corrected evidence/build-dependent-compiler.py for rebuilt transitive dependency search paths. Existing authoritative result paths are refused on rerun.

The objective remains active until Mech meets or beats the fastest eligible Rust.
