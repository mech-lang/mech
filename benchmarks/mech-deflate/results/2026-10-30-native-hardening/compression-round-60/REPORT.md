# R60: static vector address generation

Candidate compiler: `aea11e1f17f05055535171fa1b0cb8b5ff2d4bcf`. Baseline R59 compiler: `09b53c3d57f52fef8d9a067b9f110f48c0ddbafc`. Only GPU was rebuilt; every other production library hash, feature set and profile is unchanged.

**Retired instructions improve, with adverse latency and binary-size tradeoffs:** elapsed increased 0.3228%, cycles increased 0.1961%, while retired instructions decreased 0.9686%. This is not an elapsed-time or binary-size win. The candidate is retained for the independent instruction objective; generated-code growth remains unresolved.

The complete eight-byte level-12 call measured **13.565443× the reference**, from **13.521790×** in the matched baseline cohort. Candidate/baseline elapsed is **1.00322840×** (+0.3228%). Reference median: 0.283000 ms.

The source-level change uses direct linear addressing when the existing numeric proof establishes a unit row or column dimension; general rectangular geometry retains its normal mapping. This changes emitted arithmetic, not the compressor algorithm or its logical resource charges.

| Metric | Reference | R59 baseline | R60 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.283000 | 3.826666 | 3.839021 | +0.3228% |
| Retired instructions | 1,364,372 | 42,850,940 | 42,435,876.5 | -0.9686% |
| CPU cycles | 907,293 | 12,097,624 | 12,121,351.5 | +0.1961% |
| Allocation requests | 95 | 8,173 | 8,173 | +0.0000% |
| Requested bytes | 9,021,923 | 98,228,396 | 98,228,396 | +0.0000% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental peak live usable bytes | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,761,984 | 319,619,072 | 319,832,064 | +0.0666% |
| Physical footprint after call | 14,402,432 | 228,755,136 | 228,902,656 | +0.0645% |

Primary elapsed, CPU counters, allocation counters and sampling are separate serialized cohorts. Their percentages are not multiplied or combined into a synthetic result. Requested bytes measure allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are distinct measures; retained compiler/catalog/image storage is included.

## Matched pairs and contrary observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.840458 | 3.844063 | 1.00093856× |
| 1 | 3.822125 | 3.840292 | 1.00475312× |
| 2 | 3.801645 | 3.844375 | 1.01123974× |
| 3 | 3.842395 | 3.827625 | 0.99615591× |

1/4 independently prepared pairs and 20/64 alternating blocks improve. Each pair has 16 blocks ×31 measured calls per variant: 3,968 total primary observations. Individual blocks within a prepared pair are descriptive, not independent experiments.

Counter directions that increased are retained below; an empty object means no listed median increased:

```json
{
  "primary_seconds": 0.32283973531532784,
  "cycles": 0.19613355482035466,
  "rss_after": 0.0666393274554089,
  "physical_after": 0.06448816956834413
}
```

## Generated binary

| Metric | R59 baseline | R60 candidate |
|---|---:|---:|
| Generated text bytes | 13,046,668 | 13,806,352 |
| Machine entry stack bytes | 108,368 | 108,880 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

| Static instruction class | R59 | R60 | Delta |
|---|---:|---:|---:|
| UDIV | 1,825 | 101 | -1,724 |
| MSUB | 878 | 16 | -862 |
| static_instructions | 3,261,667 | 3,451,588 | +189,921 |

Text grows by 759,684 bytes (5.8228%) and entry stack by 512 bytes. Opcode deltas include LDR +66,154, MOV +65,922, and MOVK +66,242. The actual Object hashes and full disassembly census are in [binary-audit.json](binary-audit.json). These are static counts; retired instructions are measured separately.

The candidate Object is independently bound to its corresponding image and libraries. Equality with the baseline Object is not an acceptance requirement.

## Diagnostic profiles and allocation census

| Sampling measure | Baseline | Candidate |
|---|---:|---:|
| Total exclusive sample weights | 1320 | 1317 |
| frame actions | 320 (24.242%) | 328 (24.905%) |
| tail cleanup | 60 (4.545%) | 42 (3.189%) |
| projection accounting | 45 (3.409%) | 36 (2.733%) |
| thread-local lookup | 47 (3.561%) | 33 (2.506%) |
| memory move/copy | 30 (2.273%) | 28 (2.126%) |

These ancestry groups overlap. They are sampling weights, not retired-instruction attribution; host sample movements do not establish a host optimization when only GPU code generation changed.
The complete candidate census reconciles all 8,173 requests, 98,228,396 requested bytes and 162 reallocations with the separate heap counters. No allocation reduction is claimed.

## Correctness and unchanged workload

The input is eight bytes `abcdefgh`; exact raw output remains `[75,76,74,78,73,77,75,207,0,0]`, with independent inflation. Output capacity remains 13 and is cleared/reused. The tree contains 262,144 U16 entries, the cache 1,500,513 U32 words, search depth is 300 and nice length 258. Input aliases, finite admission/error order, previous publication on failure and recovery remain covered. Logical work is 60,260,219,686 and comparison work 47,612; neither is a hardware instruction count. Final managed bytes and nodes remain zero.

The source and bytecode are unchanged:

```json
{
  "source_sha256": "331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9",
  "bytecode_sha256": "a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7",
  "allocator_sha256": "38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874",
  "logical_work": 60260219686,
  "comparison_work": 47612,
  "input_bytes": 8,
  "expected_output": [
    75,
    76,
    74,
    78,
    73,
    77,
    75,
    207,
    0,
    0
  ],
  "output_capacity": 13
}
```

## Qualification

All required local checks passed: **True**. Exact counts come from archived inventories and executed receipts. Batches overlap; they are not a sum of unique tests.

| Receipt | Expected tests | Observed | Passed |
|---|---:|---:|---|
| `units-after/build.json` | None | None | True |
| `units-after/vector-address.json` | 1 | 1 | True |
| `units-after/preparation.json` | 2 | 2 | True |
| `units-after/runtime.json` | 327 | 327 | True |
| `units-after/work-values.json` | 32 | 32 | True |
| `qualification/component-build.json` | None | None | True |
| `qualification/path-mechanism.json` | 1 | 1 | True |
| `qualification/components.json` | 236 | 236 | True |
| `qualification/composition.json` | 8 | 8 | True |
| `ci/workflow-python.json` | 132 | 132 | True |
| `linked-acceptance/emit-build.json` | None | None | True |
| `linked-acceptance/emit.json` | None | None | True |
| `linked-acceptance/worker-build.json` | None | None | True |
| `linked-acceptance/run.json` | None | None | True |
| `ci/results.json` | 33 | 33 | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `primary/scalar-results.json` | — | 12 | True |
| `frame-allocation/results.json` | — | 8 | True |
| `linked-acceptance/run.stdout` | — | 12 | True |
| `parent-cases/cases.json` | 2 | 2 | True |

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local exact inventory and architecture checks reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization is measured on the complete pinned eight-byte level-12 call only.

Preflight attempts (both preserved, neither contributed a timing sample):
- Exit 1, observations 0: Clean-worktree preflight rejected the untracked R59 reconstruction.json written by analysis agent; no reference or compressor timing ran. Receipt committed before continuation.
- Exit 1, observations 0: Analysis agent completed its already-running audit copy before stop message arrived. Clean-worktree guard rejected that untracked audit directory. No timing calls ran; audit committed before continuation.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from this evidence packet. It verifies every archived receipt version, overlays versions chronologically and reproduces scorecard, profile and validation JSON. No compiler, compressor executable or performance rerun is required.

This is a small-input complete-compression result. It does not establish larger-input throughput or a standalone minimum memory footprint. The candidate is retained for reduced retired instructions despite its larger generated text and stack; the cause of this growth remains under investigation. Elapsed and cycle medians are adverse; allocation counts are unchanged. No R61 result is combined with this decision.
