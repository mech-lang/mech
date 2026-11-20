# Admitted input words and packet lifetime combinations

**The qualified selection remains 63,612 B, 81/81 Roc and 62/81 libdeflate. There is no 63rd C win.** This study preserves nine experimental variants and all eighteen forward/reverse discovery orders. Each order has twenty-one cases and twenty-four balanced blocks per case. Every previously required peer win observed in this panel remains a point win. This does not establish a full 81-case score or confidence qualification for these variants.

The word candidate remains **63,292 B**, with peak stack reduced from 4,832 to **4,736 B**. The combined word-probe/vector candidate is **63,756 B / 4,736 B stack**. Admitting one full input word removes two invariant input-length predicates and their saturated-window selections. The existing span guard checks `length >= 8` before phase effects; short inputs use the original checked continuation. Signed predicates, unknown or changing lengths and proof misses remain unchanged. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added.

Source for the word-floor mechanism is **`baeab0027d90305fa3a9b750264acf621acf94ce`**. Timing compiler SHA is **`4aeff7086e0d024394a55a5068704bab9735df49525b5a6118e2b91146fefcc5`**. Source **`6062e2e65`** adds artifact-preserving optimized IR inspection; its compiler reproduces the same qualified reference and experimental artifacts exactly.

## Runtime discovery

| Candidate | Encoder text | Natural 4 KiB / C, forward | Reverse | Time vs held-feature base, forward / reverse |
| --- | ---: | ---: | ---: | ---: |
| Input-word, scalar words | 63,292 B | 1.028202× | 1.030644× | 0.987905× / 0.999798× |
| Input-word + first-word probe | 63,756 B | **1.009970×** | **1.022278×** | **0.987917× / 0.996594×** |
| Probe + qualified packet loads | 63,340 B | 1.033509× | 1.042713× | 1.003305× / 1.019937× |
| Probe + pinned immutable loads | 63,756 B | 1.027836× | 1.021227× | 1.001194× / 0.996011× |

The first two rows compare word-floor-on against the matching prior span/probe recipe. The last two compare against the input-word/probe worker. These are separate cohorts; their C ratios must not be subtracted to claim causal gains. The input-word/probe row still needs about **0.99–2.18% less candidate time** to reach C parity. Larger-block, full-case qualification is outstanding.

The smaller word variant's reverse `320-overlap` estimate is **0.999809× C**, so that prior win remains fragile. The probe variant is **0.981041× / 0.976482× C** there. Every observation remains included, including small-case slowdowns that preserve peer wins.

Revisiting immutable packet loads saves **416 B** from the probe candidate, but makes natural inputs slower and costs about **9.5–9.6%** on distribution change. It is retained for repair. Deferring multiply used exit packet words and combining both mechanisms also fail to establish a new C win. All five packet variants and their results are in `result.json`; none is silently discarded.

Four initial packet emissions failed because old profile offsets no longer matched. Raw emission showed identical ordered control-flow graphs. The successful source builds rebase each profile only after checking exact ordered blocks, transfer destinations, literal-pool boundaries and padding. These are layout heuristics, not latency predictions. The failed attempts and graph proofs remain archived.

The untimed scalar-word collector diagnostic removes **9,087 instructions** on natural 4 KiB relative to its previous span body. This does not establish a proportional whole-encoder saving. Diagnostic clones are never timed as deployment workers.

## Complete builds on Apple M1

| Actor | Encoder text | Source through final executable link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Qualified reference, same compiler | 63,612 B | 2.212362 s | 2.223302 s | 276,072 B |
| Input-word, scalar words | 63,292 B | **2.213294 s** | 2.224221 s | 276,072 B |
| Input-word + first-word probe | 63,756 B | 2.216433 s | 2.227288 s | 276,072 B |
| Probe + qualified packet loads | 63,340 B | 2.214793 s | 2.225910 s | 276,072 B |
| Roc | 57,696 B | **2.983196 s** | 2.994080 s | 191,912 B |
| libdeflate | 37,080 B | **0.480611 s** | 0.491591 s | 155,752 B |

Twelve balanced serial fresh-process trials per actor include final executable linking. Compilers and fixed support are prebuilt equally. Strip and compiler bootstrap are separate. Every emitted Mech object, complete metadata and bytecode must match its recipe. No runtime acquisitions, test builds, counters, profiling or archive compression run during this cohort.

The word candidate's paired complete-build ratios are **0.742082× Roc / 4.611124× C / 1.000973× the qualified reference**. Probe ratios are **0.743023× Roc / 4.615789× C**. This study establishes no compile-time gain.

Word-candidate phase medians are **701.337 ms parsing**, **214.166 ms source semantics**, **6.838 ms artifact preparation**, **2.539 ms encoding**, **1,227.509 ms native emission**, and **34.947 ms final linking**. Catalog work overlaps parsing; independent medians do not add. The 206.6-second Rust rebuild for the inspection compiler is a separate producer-build record, not Mechdeflate compilation time or a matched compiler-bootstrap comparison.

## Actual optimized IR and validation

The older `MECH_INSPECT_PHASE_CLIF` selects an observed compilation pipeline and can change admission or layout. Its failed attempt is retained. New **`MECH_INSPECT_OPTIMIZED_PHASE_CLIF=/path/mech_compact_numeric_475_vector.clif`** writes the prepared body without changing observation mode, admission, disassembly or compilation sharing. Twelve plain/inspected/checked-inspected source replays cover the qualified reference, word, probe and packet-qualified variants. All preserve the exact object, complete metadata and canonical bytecode.

The actual collector still checks overflow and input extent for `position + min(length - position, limit)`. The admitted span establishes the needed position relation. This is a concrete next proof target; no saving from removing those checks is claimed here.

The word-floor source passes **21 relevant repository tests**: four admission tests, two histogram tests, ten equality tests and five packet-storage tests. Span and histogram tests execute **34,440 native differential calls**. The complete GPU suite is not claimed. All nine timed variants pass 162 output/inflation/alias observations, six checked failures and the 5,280 B stack cap. Word and probe pass twelve injected failures followed by twelve exact recoveries each. All five packet variants reproduce their artifacts with checked register allocation. Public relocated profiles reproduce word and packet-qualified artifacts exactly in normal/checked modes.

The repository-wide unsafe audit still has the pre-existing missing allowlisted archived allocator. No scalar entry, error origin, ownership check or resource accounting is retired. These checks use the documented diagnostic entry; they do not establish ordinary managed-entry admission and recovery.

## Reproduce and verify

Use a minimal compiler built from PR #995 at `6062e2e65` or a compatible later source. The exact producer command, configuration and source inputs are in `checks-and-builds.tar.gz`.

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-input-word-replay
python3 reproduce.py /path/to/compile_profile /tmp/mech-packet-replay --role packet-qualify-profiled
python3 verify.py
```

The verifier checks source and artifact hashes, all archive members, repository-test receipts, actual linked-worker and stack gates, recoveries, all eighteen raw paired discovery orders, inspection identity and twelve complete-build trials per actor. Every adverse observation is retained. Different hardware or support requires fresh runtime qualification.
