# IR304: completed generic dictionary native through lookup

**Retained processing improvement: confirmation is 1.885625 ms / 1.233241× fastest eligible Rust (1.529000 ms).** IR303 in the same cohort is1.904167ms/1.245368×. The remaining gap is356.625µs, requiring18.91% less current processing time. Cranelift remains production; no LLVM backend or new LLVM dependency.

The current compiler proves one completed generic String/U64 owner is consumed only by an already-proved native UTF8 comprehension, retains its actual HashMap/Box<str> ownership, and hands an immutable lookup loan to emitted code. Canonical dictionary publication is omitted. Preparation, finite admission, and lifetime helpers remain linked native runtime code; this does not claim that all preparation has become direct emitted instructions. The seed tuple/import round trip remains a measured next compiler gap.

| Implementation | First processing | × fastest Rust | Confirmation processing | × fastest Rust |
|---|---:|---:|---:|---:|
| Fastest eligible Rust | 1.530083 ms | 1.000000× | 1.529000 ms | 1.000000× |
| IR295 retained control | 1.935542 ms | 1.264992× | 1.920125 ms | 1.255804× |
| IR297 constructor control | 1.948708 ms | 1.273596× | 1.947667 ms | 1.273818× |
| IR302 private-write control | 1.936958 ms | 1.265917× | 1.926292 ms | 1.259838× |
| IR303 immediate/selected parent | 1.914500 ms | 1.251239× | 1.904167 ms | 1.245368× |
| IR304 canonical-dictionary emission control | 1.908458 ms | 1.247291× | 1.916667 ms | 1.253543× |
| IR304 native completed dictionary | 1.884250 ms | 1.231469× | 1.885625 ms | 1.233241× |

Against IR303, median paired processing changes are−1.1000% (95%[−2.2647%,−0.3884%],25/31 favorable) and−0.8886% ([−1.4402%,−0.1170%],23/31) in independent cohorts. Against same-runtime emission ablation, the first interval excludes zero (−1.1028%,[−1.7487%,−0.2636%]); confirmation does not (−0.9067%,[−3.0714%,+0.7751%],17/31). This attribution uncertainty is retained. All22 Rust controls remain,20 eligible for continuous post-JSON processing; preinitialized worker pools are excluded only from that boundary. Each cohort contains31 fresh randomized latency samples×29 implementations and7 separate resource samples×30 implementations. All1,798 latency and420 resource outputs are exact. An earlier549-row cohort was interrupted by storage exhaustion and is retained separately as incomplete/ineligible, with unchanged retry seeds and source.

Processing includes input preparation/admission and allocations, emitted computation, worker creation/run/join, internal cleanup and result materialization. JSON decoding and artifact restoration are separate and included in whole process. Resource counters are not latency qualification.

## Resource and deployment costs

| Confirmation processing resource | Fastest Rust | IR303 | IR304 |
|---|---:|---:|---:|
| Allocation requests | 58 | 1,291 | 1,145 |
| Requested bytes | 6,825,713 | 7,807,730 | 7,800,403 |
| Additional peak requested heap bytes | 5,648,124 | 5,765,741 | 5,763,534 |
| Total peak requested heap bytes | 7,703,209 | 14,109,942 | 14,107,901 |
| Instructions, all workers | 17,683,240 | 33,930,945 | 33,626,694 |
| Cycles, all workers | 5,971,110 | 11,217,908 | 11,224,731 |

IR304 saves146 processing requests (11.31%),7,327 requested bytes and2,207 additional peak heap bytes versus IR303. Confirmation instructions fall304,251 (0.897%); first fall121,496 (0.357%). Cycles do not show a repeatable reduction: first rises344,708 and confirmation6,823. The same-runtime control has1,292 requests; enabled lowering removes147 of them. All resource observations remain visible. Native generated code/stack stay1,031,248B/1,584B (maximum numeric function frame944B). The original latency executable grows29,296B to17,127,024B. Requested heap, process RSS/physical footprint, generated frame sizes and executable deployment costs are different metrics; full process counters remain in raw resource reports.

Whole-process medians rise38.468750→41.459208ms, median paired+9.1503% (95%[+6.4948%,+9.6862%]). Restoration rises8.598333→12.208375ms,+41.7320% paired ([+41.4707%,+42.4308%]). The regression also occurs with native-dictionary emission disabled; it is not a processing-timer exclusion. Re-derivation of the additional generic proof is a candidate cause, not a completed causal profile. Whole process is not an acceptance gate under the user's instructions.

## Ownership, fallback, and qualification

The proof excludes public/structural aliases, multiple consumers, unsupported activation handoffs, non-U64 maps, and non-linear work between publication and lookup. Metadata restoration independently rederives the proof. No28-key/short-string/empty-seed assumption is used. The table owns its existing stable Box<str> keys and values; the original finite account, logical output/clone obligations, spent relative-call prefix and extra metadata/node reservations stay attached. Lookup preparation transfers ownership only after fallible storage allocation succeeds. A preparation refusal or native loop miss drops all pointers/scratch first, then consumes the dictionary once through ordinary canonical publication using the source operation's remaining allowance. Virtual source generations, current table/account, stale handles and output atomicity are checked. Enclosing external relative scopes retain immediate canonical publication.

Required qualification:460 native contracts, retaining all451 IR303 names;173 retained Map tests;29 image/caller tests; inherited135 Engine contracts with only existing has_active_call_limit visibility widened and body byte-identical; new3 storage API and4 runtime contracts plus2 applicability proofs; empty/Unicode/NUL/73-long-key cases, original/decoded/restored execution, exact/one-less relative/global budgets, missing-key fallback, stale generations, aliases, failed-output preservation, cleanup and retry. All128 scalar and5 DEFLATE cases retain every nontiming field, including allocations and work. Default and emission control each pass32 linked output cases and exact native-code audits. Full5000 original/restored observation confirms1 native dictionary publication,0 canonical dictionary Values,1 frozen lookup loan,0 fallback materializations, while retaining1 owned key view/28 borrowed cells/0 canonical key matrices or String values.

Expanded collection coverage finds62 passes and5 failures identical in IR303 and IR304: four existing ASCII joined-string StaleHandle errors and one unavailable stats import in this minimal dependency build. Both complete failing receipts and the attribution comparison are retained; no selection was disabled and no semantic test weakened. These existing gaps remain open.

## Fresh profiles and next work

The fresh production Object sample verifies code identity and exact final output across10,001 repeated fresh invocations. A separate64-output aligned phase capture compares current default, same-runtime ablation, and fastest Rust; all traces have balanced checkpoints and counters. Probes affect execution, phase medians are not additive production savings, and inclusive sampling stacks are not CPU percentages.

| Aligned diagnostic phase | IR304 median | Rust median | Paired Mech−Rust |
|---|---:|---:|---:|
| Preparation/interning/eligibility | 801.666µs | 592.896µs | +215.354µs |
| Triple index/coverage | 353.563µs | 181.979µs | +172.333µs |
| Quartet index/masks | 263.812µs | 196.334µs | +68.937µs |
| Queries/cleanup/materialization | 478.521µs | 561.416µs | −78.771µs |

Mech loses all8 preparation/triple/quartet pairs and wins7/8 query pairs against Rust. Against same-runtime ablation, preparation has5/8 favorable pairs and a−20.688µs paired median; probes show numeric-phase variation despite unchanged kernels, so no sum is used as the production effect. Preparation requests fall934→787 (Rust38); detailed dictionary prelude590→443. Publication site78 falls147→0 requests and20.167→3.084µs diagnostic time.

**Next concrete gap: source44 core/composite-pack still creates the private native seed frame at406 allocation requests,74,719 requested bytes and92.084µs diagnostic time.** It accounts for406/443 remaining dictionary-prelude requests. Seed import site45 adds37 requests/~18.625µs;28 borrowed-key reads and insertions take about26.542/29.752µs with0 requests. Keep this seed frame/native nonempty seed through owner initialization without constructing canonical intermediate aggregates; preserve genuine snapshot ownership, current values vs immutable keys, exact finite/logical source obligations and one-shot ordinary fallback. Histogram native work and numeric index construction are also still substantial; completed-producer element range into scatter remains a separate attributable compiler task.

Source commit: [ebaac8e7944d24bbb6c1938a3514cde86511711d](https://github.com/mech-lang/mech/commit/ebaac8e7944d24bbb6c1938a3514cde86511711d), branch codex/related-generic-native-dictionary-20261108. PR990 remains targeted at the PR973 branch with its separate historical integrated IR242 head. This research source is published separately.

## Reproduction

measurements/ contains both complete latency/resource cohorts and the interrupted first partial report. evidence/ preserves raw schedules, outputs, paired analyses, qualification, all first/adverse attempts, native sampling and the64 aligned diagnostic captures. runners/, builds/, regressions/ and tools/ preserve pinned source/link/dependency recipes and inventories. Rust sources named .rs.txt preserve exact bytes without changing the frozen Rust compiler inventory. Named duplicate output files become .json.ref.json aliases with exact SHA/byte count and a shared byte-for-byte blob. Build executables were losslessly archived after completion with SHA/mode manifests; authoritative runners remain raw. Zero-byte metadata-only check outputs were removed during storage recovery, while all source, receipts, executable archives and measured output data remain.

Recreate pinned dependency paths, then use evidence/run-cohort.py and evidence/run-cohort.py confirmation. Existing completed result paths are rejected. The prospective rule, seeds, all first/adverse samples and whole-process costs are retained. The goal of meeting or beating Rust remains active.
