# IR305: native dictionary seed frames

**Selected processing:1.768708 ms /1.151190× fastest eligible Rust1.536417 ms.** Same-cohort IR304 is1.854833 ms/1.207246×. Remaining gap232.291 µs requires another13.1334% reduction in current Mech processing. Cranelift remains production; no LLVM dependency was added.

The compiler proves a closed flat U64/generic String–U64 Map seed is consumed only by adjacent retained-owner initialization. It retains genuine admitted child headers and skips the canonical aggregate. Original/restored5000-post execution selects 1 native seed frame,0 canonical seed tuples,1 direct import and 0 late materializations. Existing borrowed keys and completed native dictionaries remain active. The Mech source and four query workers are unchanged.

## Processing, both independent cohorts

| Implementation | First ms | × fastest Rust | Confirmation ms | × fastest Rust |
|---|---:|---:|---:|---:|
|mech-ir295-histogram-reuse-4|1.911375|1.243508×|1.904709|1.239708×|
|mech-ir297-closed-constructors-4|1.928708|1.254785×|1.914500|1.246081×|
|mech-ir302-private-write-first-4|1.901750|1.237246×|1.912875|1.245023×|
|mech-ir303-borrowed-map-key-views-4|1.864875|1.213256×|1.871792|1.218284×|
|mech-ir303-canonical-map-key-control-4|1.900167|1.236216×|1.903958|1.239220×|
|mech-ir304-generic-native-dictionary-4|1.854000|1.206181×|1.854833|1.207246×|
|mech-ir304-canonical-dictionary-control-4|1.875750|1.220331×|1.871334|1.217986×|
|mech-ir305-native-dictionary-seed-frames-4|1.777917|1.156682×|1.768708|1.151190×|
|mech-ir305-canonical-seed-control-4|1.854209|1.206317×|1.857042|1.208684×|

Fastest eligible Rust is1.537083 ms initially and 1.536417 ms in confirmation, both rust-ordered-mask-tables256.

| Paired processing comparison | First change,95% interval | Confirmation change,95% interval | Favorable pairs first/confirmation |
|---|---:|---:|---:|
|IR305 vsmech-ir304-generic-native-dictionary-4|-4.3224% [-4.7532%,-3.5648%]|-4.5966% [-5.0926%,-4.1930%]|31/31;31/31|
|IR305 vsmech-ir305-canonical-seed-control-4|-4.4129% [-4.9948%,-3.8471%]|-4.6841% [-5.2872%,-4.2640%]|31/31;31/31|
|IR305 vsmech-ir295-histogram-reuse-4|-7.7179% [-8.0188%,-6.9032%]|-7.1740% [-7.6587%,-6.5960%]|31/31;31/31|

The same-runtime control disables only seed-frame producer emission and uses the identical Engine/runtime. Its processing comparison with IR304 crosses zero in both cohorts. The candidate improvement is independently confirmed and attributable to lowering.

Each cohort retains 31 implementations:all 22 Rust controls,20 eligible for matched processing, and 9 Mech controls.31 randomized fresh-process latency samples per implementation plus 7 separate resource samples×32 variants give 1,922 timing and 448 resource exact outputs. All first/adverse observations and prospective seeds remain. No incomplete IR305 cohort exists.

The authoritative timer is one continuous post-JSON interval covering input preparation, admission, allocations, native execution, worker create/run/join, internal cleanup and materialization. JSON IO and artifact restoration are separately recorded and included in complete process. Preinitialized Rayon pools are ineligible only for matched processing. No builds, tests, profiles, archives or pushes overlapped either timed cohort.

## Resource and deployment costs

| Confirmation processing resource | Fastest Rust | IR304 | IR305 |
|---|---:|---:|---:|
|Allocation requests|58|1,145|749|
|Requested allocation bytes|6,825,713|7,800,403|7,727,508|
|Additional peak requested heap bytes|5,648,124|5,763,534|5,760,767|
|Total peak requested heap bytes|7,703,209|14,107,905|14,105,394|
|Instructions, all workers|17,661,863|33,587,099|33,088,987|
|Cycles, all workers|5,883,156|10,877,187|10,429,647|

IR305 saves396 requests (34.59%) versus IR304 and 397 versus emission ablation;72,895 requested bytes and 2,767 additional peak heap bytes versus IR304. Instructions fall 518,192 initially and 498,112 in confirmation. Cycles fall 463,438 and 447,540. Resource adapters are separately instrumented; their latency is ineligible. Requested heap differs from process RSS/physical footprint; raw reports retain those process counters plus CPU time/faults.

Generated native code shrinks80 B to 1,031,168 B; the largest generated frame grows 16 B to 1,600 B, while the largest numeric frame stays 944 B. The latency executable grows 240 B to 17,127,264 B. Same-Object code audits pass.

**Whole process regresses and remains visible:** confirmation43.420959→46.943500 ms; median paired+8.9556% (95%[+6.2475%,+10.1906%]). Restoration12.144875→15.696458 ms; paired+29.3483% ([+28.8760%,+29.6537%]). The canonical-seed control also pays the restoration increase. Added proof rederivation is a candidate cause; causal profiling remains open. Whole process is not an acceptance gate under the user’s instructions.

## Semantic and finite-resource qualification

The proof requires exact single-definition CompositePack arity 1–32, a closed flat tuple/record of U64 fields and one String/U64 Map, a sole LoopSeed use, exact parent/dominance/activation facts, and direct adjacent producer/initializer control. It is freshly rederived from checked metadata on restoration, never accepted as serialized authority. Both actual owner regions are covered. Empty, Unicode, NUL and 73-long-key fixtures prove there is no28-key, short-key or empty-seed assumption. Escaping/public/unsupported uses retain ordinary construction. Histogram counts and dictionary IDs remain different values.

NativeMapSeedFrame owns genuine admitted children, exact canonical logical bytes/nodes, original factory/schema/account identity, finite clone/output obligations, real metadata/node claims and the spent original call prefix. Complete coexistence geometry includes private owners and borrowed views. A later owner refusal consumes the frame once through the original constructor using its remaining relative allowance. Intervening global work remains charged; no fresh allowance is granted. Child headers move to bounded stack storage and the pin allocation retires before its claims. No fake Value or borrowed raw-pointer lifetime shortcut is used. Ownership/admission helpers are linked native Rust runtime code; aggregate materialization is omitted. Scalar Bool/U64/I64/F64 lowering remains maintained.

Required qualification:466 actually inventoried/executed native CI contracts, preserving all 462 prototype names and 460 IR304 names;173 retained Map tests;29 image/caller tests;28 additional required UTF8/range/protocol tests;138 Engine native collection tests (all 135 inherited plus 3 new);19 projection/frame/relative-budget regressions. New tests exercise original/decoded/restored execution, public Map aliases, wrong factory/account, stale generations, malformed managed words, immediate canonical refusal, later owner refusal, exact/one-less finite limits, first errors, output atomicity, full byte/node cleanup and retry. Exact fallback relative allowance22,862,024 and global22,862,061 with 37 intervening units are recorded for both artifact forms.

All 128 scalar and 5 DEFLATE cases match IR304 in every nontiming field, including physical allocations/requested bytes and work/visits. Candidate and emission control each pass32 linked correctness cases. Engine source 405files matches the test inventory; final production/runtime inventories match the qualification snapshot.

Expanded coverage retains 62 passes and the same5 failures observed in IR303/IR304:four existing ASCII joined-string StaleHandle errors and one unavailable stats-module import in this minimal dependency build. Full receipts/failure names are retained; no tests were disabled or weakened. Those gaps remain open. First failures for a missing tracked architecture fixture, a missing transitive dependency search path and unused_mut remain in evidence. The final successful dependent production GPU build isv4; partial Engine-foundation recipes are explicitly labeled, not passed off as a complete compiler.

## Fresh profiling and next work

A new sample covers10,001 fresh native invocations of the exact production Object with exact final5000 output. Inclusive stacks overlap and are not CPU percentages. A separate64-output aligned capture uses unchanged production Engine and Objects, eight randomized pairs, coarse/detailed probes and ordinary controls. All captures match Rust; coarse boundaries and counters balance. Probe timings are diagnostic and phase medians are not additive production savings.

| Aligned phase | IR305 µs | Rust µs | Median paired Mech−Rust µs |
|---|---:|---:|---:|
|preparation interning and eligibility|743.938|607.229|+132.167|
|triple index and coverage|350.979|185.417|+162.250|
|quartet index and masks|265.000|208.438|+58.583|
|queries cleanup and materialization|486.375|577.750|-93.438|

Preparation beats the seed-emission control in8/8 pairs (paired−87.772 µs); preparation requests fall 788→391, versus Rust38. Detailed dictionary-prelude requests fall 443→46. Source 44 falls406→9 requests and 97.626→10.208 µs. Source 45 import still needs 37 requests/~19.271 µs.28 borrowed-key reads and 28 inserts remain allocation-free but take~27.961/29.291 µs in detailed callback captures. Necessary checks and probe overhead are included.

Triple construction is now the largest aligned deficit (8/8 losses), followed by preparation (7/8 losses) and quartets (7/8 losses); queries win8/8. Evaluate the completed-producer key-range proof into scatter as a separately attributable compiler task; it does not prove destination-position or posting-ID bounds. Also evaluate an owned immutable key snapshot with compact current dictionary values, removing copied seed import and repeated borrowed-read/insert helper checks while preserving histogram counts. Histogram finish and input/session admission remain additional targets. Restoration proof costs need causal investigation. No future speed gain is claimed.

## Every retained Rust timing, confirmation

| Rust version | Reported processing ms | × fastest eligible Rust | Matched-processing eligible |
|---|---:|---:|---|
|rust|8.795708|5.724818×|yes|
|rust-preinitialized|1.863084|1.212616×|no: preinitialized pool|
|rust-mech-control-streaming|1.628709|1.060070×|yes|
|rust-mech-control-u32-casts|1.632084|1.062266×|yes|
|rust-mech-control-u8-ids-u32|1.623084|1.056409×|yes|
|rust-ordered-scan-only|1.581125|1.029099×|yes|
|rust-ordered-ordered|1.556791|1.013261×|yes|
|rust-ordered-mask-reuse|1.557209|1.013533×|yes|
|rust-ordered-ordered-mask|1.540042|1.002359×|yes|
|rust-ordered-ordered-mask-tables|1.541833|1.003525×|yes|
|rust-apple-m3|8.803708|5.730025×|yes|
|rust-preinitialized-apple-m3|1.899625|1.236399×|no: preinitialized pool|
|rust-mech-control-streaming-apple-m3|1.636708|1.065276×|yes|
|rust-mech-control-u32-casts-apple-m3|1.626375|1.058551×|yes|
|rust-mech-control-u8-ids-u32-apple-m3|1.626583|1.058686×|yes|
|rust-ordered-scan-only-apple-m3|1.627875|1.059527×|yes|
|rust-ordered-ordered-apple-m3|1.634041|1.063540×|yes|
|rust-ordered-mask-reuse-apple-m3|1.624541|1.057357×|yes|
|rust-ordered-ordered-mask-apple-m3|1.633000|1.062862×|yes|
|rust-ordered-ordered-mask-tables-apple-m3|1.625416|1.057926×|yes|
|rust-ordered-mask-tables256|1.536417|1.000000×|yes|
|rust-ordered-mask-tables256-apple-m3|1.540708|1.002793×|yes|

Source:[32eb7450d54f32de12a70cefba0eda2f1f279be6](https://github.com/mech-lang/mech/commit/32eb7450d54f32de12a70cefba0eda2f1f279be6), branch`codex/related-native-dictionary-seed-frames-20261108`. PR990 retains its separate integrated IR242 head and targets the PR973 branch; this research is published separately.

## Reproduction

measurements/ keeps both complete latency/resource cohorts; evidence/ preserves schedules, analyses, exact output aliases, qualification, all first/adverse attempts, native samples and 64aligned captures. builds/, runners/, regressions/ and tools/ pin source/dependency/flag/link recipes. Source copies end.rs.txt so evidence does not alter the frozen compiler inventory. Repeated outputs use lossless.json.ref.json aliases with SHA/byte count and shared byte-for-byte blobs. Executables can be rebuilt from retained recipes; completed local functional binaries are losslessly archived with SHA/mode manifests. Authoritative runners remain intact.

Recreate pinned paths, then use evidence/run-cohort.py and its confirmation mode. Existing result paths are refused. The test-only applicability packet is separately preserved on this branch. The goal remains active until Mech meets or beats the fastest eligible Rust.
