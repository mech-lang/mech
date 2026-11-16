# R163: settle proved complete native child visits once

The compiler proves a bounded, immutable two-state unit-step child total under freshly admitted array windows. It debits the same existing traversal prices and records the exact visit count once per activation, removing the per-header debit/counter update only in that proved version. The parent phase was already fully funded: this is not a claim that per-candidate balance tests were previously present. The original child and finite incremental execution remain when proof or current admission declines.

The source proof checks every reachable operation and latch edge before merging states. An active entry has max(count-start,0)+2 header visits; an initially terminal entry has one. Each active continuation preserves the tag and increments k by exactly one under k<count. A noncontinuing active edge preserves k and installs the terminal tag. Only the initially terminal traversal may exit. Projection guards, source reads and fallible arithmetic must be total. Unknown effects, mutable views, cold edges, nonunit steps, early exits and unsupported operations decline. The gate checks fresh current views/tag, count/product/counter overflow and current loan credit before any debit. It reuses the original compute/comparison prices and settlement counter. Shared source emission, current authority, error prefixes, publication and cleanup remain unchanged. No permission survives a traversal or invocation.

Base 27081b4000d1f1764c14df32245bd3938984ed3f; measured compiler **19cebb4ca84a611c149ba9ac5d652156cf4042a5**, compared with qualified R162 **3ef813384ab5a1df7a90818ea89506c62ad03375** (same production tree as the consolidated parent). IR/work/ABI/MBC **184/107/39/14**. Final formatting, an added tie/wrapping test, CI count, docs and evidence are folded into this same optimization commit. The measured SHA is preserved as provenance rather than presented as that final amended SHA. Diagnostic regeneration after formatting reproduces the entire primary Object byte-for-byte.

Object SHA256 **5ee1f3ccf798a57fccb4dc80814531b70b1df83dd76612680f4bdd96e97a740c**. Generated text **8,955,288→8,894,664 B (−60,624)**; entry stack **16,208→16,224 B (+16)**. Four source-derived child copies qualify. Completion also proves72 static tag-projection guards redundant. These are static counts, not executed visits.

The actual guarded cost loop has no per-header price debit or settlement increment on its active backedge. The old guarded loop still updates its −92480 credit and header counter there. Both current loads remain direct typed loads, and prior source-helper/bounds/narrowing removals remain. Current code still contains duplicated tag/comparison control, U64 masks implementing U32 costs, two strict-min diamonds, a node-length reload and an offset-cost spill/reload. The freshly disassembled pinned reference is an eleven-instruction **scalar** counted cost loop; no SIMD is claimed for that kernel. Its natural32 executable SHA256 is40d2eb378f8fb3ee738be706924f8931a9171f4ba65f5dccddd53c4768c33ca4. Exact commands, addresses and excerpts are in instruction-inspection/full-loop-inspection.json and its .asm files.

All20 complete streams pass exact raw reference bytes, independent inflate, output prefix/capacity/clear-reuse, immutable input aliases, equal source work and final managed bytes/nodes zero. Preserve source331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9, MBCa2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7, tree262144 U16/cache1500513 U32/path305257 U64, depth300/nice258, pinned EOF rules, tables, algorithm, source allocation sites, input aliases, output clear/reuse and evaluate_into timing. Allocator SHA25638c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874. No source rewrite, unroll or SIMD is included.

The predeclared balanced cohort has **3,968 primary /4048 total complete calls**, no trimming, **20 faster /0 slower case medians**, geometric after/before **0.984406464674 (-1.559354%)**. Every unchanged1.05 per-case gate passes. Block-level contrary observations remain archived. Reference medians of zero stay below resolution; no artificial ratio is supplied.

| Case | Before ms | Current ms | Before / Roc | Current / Roc | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.356541|1.336896|4.710214×|4.642000×|-1.448205%|
|16384-distribution-change|5.894979|5.755604|7.452565×|7.276364×|-2.364300%|
|33027-periodic-window|8.305250|8.182522|6.612460×|6.514746×|-1.477722%|
|random-32769|6.762104|6.650958|6.421752×|6.316199×|-1.643667%|
|repeated-32769|7.920708|7.815104|5.955420×|5.876018×|-1.333271%|
|reymont-prefix-32769|23.274104|22.861666|5.350369×|5.255556×|-1.772088%|
|reymont-prefix-16384|12.799124|12.531417|6.025953×|5.899914×|-2.091608%|
|random-1024|1.258709|1.235354|4.113426×|4.037105×|-1.855434%|
|repeated-1024|1.512583|1.485584|4.625638×|4.543069×|-1.785025%|
|reymont-prefix-1024|1.721688|1.692416|5.217235×|5.128535×|-1.700134%|
|320-overlap|1.348396|1.308895|4.570834×|4.436934×|-2.929444%|
|random-256|1.324979|1.302854|4.537601×|4.461829×|-1.669875%|
|128-biased-dynamic|1.158438|1.136875|3.994612×|3.920259×|-1.861343%|
|random-64|1.279146|1.250792|4.441479×|4.343026×|-2.216674%|
|32-high-entropy-stored|1.250438|1.239812|4.372159×|4.335009×|-0.849703%|
|eight-overlap|1.329354|1.313563|4.599841×|4.545201×|-1.187870%|
|eight-literals-prefix|1.370521|1.345291|4.792031×|4.703816×|-1.840869%|
|seven-stored-prefix|0.120749|0.120251|below resolution|below resolution|-0.413252%|
|one-stored|0.120209|0.119708|below resolution|below resolution|-0.416774%|
|empty|0.116500|0.116166|below resolution|below resolution|-0.286266%|

Counters use a separate cohort: eight CPU and two heap observations per side for inputs larger than1024 bytes; sixteen CPU and four heap observations per side for the remaining cases. **Sixteen instruction counts fall, four rise; eleven cycle medians rise.** These observations are retained alongside the faster primary medians. CPU samples do not partition retired instructions.

| CPU cohort | Before instructions | Current instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1060427.0|1061751.0|+0.124855%|+0.235604%|
|one-stored|1113003.5|1114475.5|+0.132255%|+0.246612%|
|seven-stored-prefix|1112768.0|1114240.0|+0.132283%|-0.849403%|
|eight-literals|12666958.0|12663528.5|-0.027074%|+2.418798%|
|eight-overlap|11860013.0|11856731.0|-0.027673%|+1.041877%|
|320-overlap|12476342.0|12456767.0|-0.156897%|+1.912700%|
|eight-literals-prefix|12668751.0|12664862.5|-0.030694%|+0.652999%|
|128-biased-dynamic|10637169.0|10635414.5|-0.016494%|+0.312337%|
|32-high-entropy-stored|11248798.5|11245030.5|-0.033497%|-0.307270%|
|16384-distribution-change|73457952.0|73492906.5|+0.047584%|-1.045292%|
|33027-periodic-window|150450498.5|149277327.5|-0.779772%|-0.651116%|
|random-64|11418128.0|11414033.0|-0.035864%|-0.153371%|
|random-256|12024371.0|12018438.5|-0.049337%|+1.483219%|
|random-1024|12415184.0|12406991.5|-0.065988%|+0.605165%|
|repeated-1024|15741638.5|15685037.0|-0.359565%|+1.813826%|
|random-32769|87316049.5|87024934.0|-0.333404%|-1.061386%|
|repeated-32769|144209938.0|143043135.5|-0.809100%|+0.144538%|
|reymont-prefix-1024|17562604.5|17442928.0|-0.681428%|-1.597843%|
|reymont-prefix-16384|153097175.0|150530329.5|-1.676612%|-2.055219%|
|reymont-prefix-32769|279073791.0|274513294.0|-1.634155%|-1.832559%|

All20 retain exactly equal allocation requests, requested bytes, reallocations and incremental peak live usable bytes. This patch reduces computation; it does not claim less copying or input streaming. Requested capacity is not initialized copying.

| Current heap cohort | Requests | Requested B | Incremental peak live usable B |
|---|---:|---:|---:|
|eight-literals|3354|17643969|15279752|
|16384-distribution-change|5679|24105386|18702352|
|reymont-prefix-32769|6449|24793116|18722528|
|repeated-32769|3135|15712250|15313264|
|33027-periodic-window|3679|15798298|15314928|

All12 scalar controls pass the same1.05 gate, with equal requests/bytes and no iteration allocation slope. Ratios range0.995412519–1.000056432; the I64-original median rises0.005643%.

| Separate complete-call CPU samples | Generated exclusive | Host callbacks/cleanup | Outside |
|---|---:|---:|---:|
|128-biased-dynamic|12.450%|74.068%|13.481%|
|eight-literals|12.921%|77.182%|9.897%|
|reymont-prefix-32769|84.197%|14.143%|1.660%|
|repeated-32769|84.933%|13.000%|2.067%|
|33027-periodic-window|80.854%|17.011%|2.135%|

Two additional five-second exact-PC profiles select the next target. Running worker leaf samples are mapped through a byte-identical Object and current linked load address. Unmapped source origins remain explicitly unattributed. Default sample output merges PCs by symbol and cannot assign its whole weight to its first printed PC; the exact-PC traces avoid that mistake. Raw trace containers and environment metadata are excluded from publication.

| Exact-PC CPU-time profile | Path-length child | Word extension | Native with unknown source origin |
|---|---:|---:|---:|
|repeated-32769|0.964%|12.675%|54.883%|
|reymont-prefix-32769|13.284%|0.526%|42.570%|

These mapped shares are lower bounds, not a complete instruction ledger. Natural text favors a compact ordered cost reduction; repeated input favors simplifying the extension recurrence. A roughly10% whole-call research goal would require a large reduction of the selected loop plus any confirmed surrounding cost; no such gain is promised. The next bounded compiler experiment compares scalar compact emission with ordered four-lane reduction, retaining incoming-best/earliest ties, U32 wrap, exact item encoding, zero/short/remainder paths and existing funding. One active implementation, no candidate array. SIMD and a separately pinned source experiment remain unfinished.

Cold restoration outside the primary timer is37.501618584s before and37.395335791s after in one pair. This is descriptive, not a repeated startup win. Primary compression timings exclude this setup as before.

Qualification uses nightly-2026-03-03 aarch64, warnings denied, GPU library opt3 and test/component roots opt1. Core/Engine/math/dependencies remain frozen R145 artifacts. The native feature superset is aot,jit,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime. The runtime-only profile also passes. Exact Rust commands and feature switches are retained in library-build.json, final-qualified/build.json, gpu-compatible.json and checks/receipts.json.

- 609 compatible GPU tests pass; the same seven exclusions remain.
- 21 focused original/decoded and direct/restored tests pass.
- 192 CI-inventory tests are listed, equality-guarded and actually executed.
- 236 unchanged component,8 composition and1 real-capacity path qualification tests pass, without increasing allowances or changing expected values.
- 132 workflow Python tests,15 architecture/catalog checks, runtime-only build and formatting pass.
- 12 balanced scalar controls pass.

Controls cover live aliases, current view changes, zero/partial/max candidates, repeated activations, nonzero starts, incoming and earliest ties, wrapping sums, six finite boundary dimensions, native finite-funded execution, unchanged prior publication on denial and retry. Fallible division, nonunit stride and early terminal variants decline completion. Parent/candidate finite-funded receipts match16 observations exactly within each configuration. Finite and unlimited setup work are not asserted equal: that difference reproduces in the clean parent.

Repaired failures are preserved: an optional interned-zero lookup initially caused a unit-test unwrap failure (corrected without changing the compressor Object); a finite-versus-unlimited equality expectation was invalid in the clean parent; the first test checked a segment rather than phase counter; module declaration ordering failed formatting and then passed. The clean parent fails the feature-specific prepaid-metadata expectation as expected. The initial exact-PC tool invocation could not initialize its normal cache under restricted access; the authorized normal-cache invocation succeeds. An eleven-versus-twelve-line reference excerpt assertion was a diagnostic-script error; the corrected script finds the actual backedge. No unresolved compiler correctness failure remains. Earlier failed stdout may have been replaced by successful resumption; exit receipts and explanations remain.

Hosted ordinary CI is cancelled by standing instruction, so there is no green hosted-CI claim. #973 remains draft pending green CI and clean exact-head review. Fresh dependency/all-feature/minimal graphs, the seven inherited GPU exclusions, an extra standalone AOT suite, full multi-MB timing and the separate ten-workload suite were not run. The actual restored linked compressor is exercised for every matrix case.

Evidence records exclude native binaries, bytecode, raw trace stores, copied user requests and private environment metadata. Fourteen unpublished archives were repacked only to remove copied assignment text, retaining every other file byte; the old originals remain local. The public omission receipt records both hashes. Numerical replay validates and reconstructs these results without compiling or compressing.

Decision: **Retain complete-child exact existing-price prepayment: all twenty unchanged case gates and twelve scalar gates pass, with 1.5593535 percent geometric elapsed improvement. Preserve contrary instruction and cycle cohorts.** Parity is not achieved. The current-source windows, simpler scalar path and exact existing-price settlement are complete; ordered reduction/SIMD and the separate source experiment remain.

Run checkpoint.py INPUT OUTPUT to regenerate results.json and REPORT.md.
