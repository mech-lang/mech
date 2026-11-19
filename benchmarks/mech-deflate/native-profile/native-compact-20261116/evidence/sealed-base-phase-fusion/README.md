# Sealed base phase fusion

**The compiler combines deterministic sealed-input parser phases and checks the work allowance before removing a pending frame.** The selected encoder remains **63,692 B**, with **81/81 Roc and 62/81 libdeflate wins in both full orders**. The new compiler reproduces its native object, complete metadata, canonical bytecode and fixed-support worker byte for byte. No new runtime campaign or 63rd C win is claimed. Original Mech source is unchanged. Existing [qualification scope and ordinary resource-accounting limitations](../../QUALIFICATION.md) remain unchanged.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Before, same selected encoder | 63,692 B | 2.212686 s | 2.223608 s | 276,072 B |
| Sealed parser combination | 63,692 B | 2.205340 s | 2.216274 s | 276,072 B |
| Roc | 57,696 B | 2.988178 s | 2.999173 s | 191,912 B |
| libdeflate | 37,080 B | 0.480449 s | 0.491357 s | 155,752 B |

Eight balanced fresh-process trials per actor include final linking, with fixed support prebuilt for every actor. The paired complete-build saving is **8.203 ms**, 90% interval **4.951–11.409 ms saved**, 8/8 faster (**0.371%**). Paired complete-build ratios are **0.738040× Roc / 4.589692× C**. Compiler bootstrap is excluded for all actors; its diagnostic log is retained separately. Strip is reported separately. Do not infer causal gains from separate cohorts.

This is parser maintenance, not the large code-size or build-time reduction still needed for 1× libdeflate. Encoder text stays **1.103924× Roc / 1.717691× C**; the C text gap is still **26,612 B**. Peak active encoder stack remains **4,848 B**, under the 5,280 B ceiling, and deployment size remains 276,072 B.

## Isolate the mechanism before promoting it

The current parser profile places **19.0%** of exclusive parser samples in base continuations. A separate diagnostic census sees **449,016 Classified** and **445,039 Raw** physical phases. First-occurrence eligible identifier captures span 53.7 ms of instrumented inclusive work, including required publication; that is not a savings forecast. The census adds **5.0%** overhead and is never used as latency evidence.

The first chain-fusion screen is tied: paired −0.610 ms, 90% interval −3.360 to +16.477 ms, 4/8 faster. Instead of discarding the mechanism, the bounded repair moves the zero-allowance check before taking ownership of the large pending enum. The combination screens 8/8 faster. A subsequent four-actor comparison separates both changes:

| Standalone uninstrumented parser | Median | Paired change versus original |
| --- | ---: | ---: |
| Original | 706.473 ms | — |
| Fusion alone | 702.779 ms | -3.651 ms |
| Allowance check alone | 703.142 ms | -3.260 ms |
| Combination | 697.916 ms | -8.351 ms |

The combination beats the allowance-only control in all eight rounds, by a paired **5.443 ms** (90% interval **4.225–6.036 ms saved**). It beats the original in all eight, by **8.351 ms** (**7.931–9.606 ms saved**). Standalone and complete-build results are separate measurements. All initial, adverse and repaired observations remain archived.

## Same grammar, logical work and yield boundaries

`Call → Classified → Raw` uses the existing handlers directly only on sealed input while each subsequent phase can pay its original allowance. It does not batch or replace the grapheme scanner. Unicode uses the same scanner; open input uses ordinary pending frames. If a grant ends between phases, the same Classified, Raw, Grapheme or result frame is materialized. Fuel, first-error ordering, markers, rule depth, event limits, token publication, identity allocation and parser work remain unchanged. The allowance check moves before popping a frame and therefore leaves the same pending stack on a zero-work poll.

**539 syntax tests pass**, including a new paired comparison of every yield on 12 lexical families, grants 1–12/4096/unlimited, finite fuel and event limits, Unicode, open input and parser suspension/resumption. Existing tests cover actual input cuts and recovery. The test-only disable switch is absent from the production continuation layout.

Every full-source parser trial reproduces tree checksum **`bfa476cac02c0fa8ab487211e366b34cf0c27b13998958a358317222cedc0056`**, including node/token identities, flags, ranges, hashes and children, with **3,967,302 parser steps and 718,353 events**. The source SHA is `653597f0…`. The final native object is `470e6899…`, full metadata `3faa1a9d…`, bytecode `c6529d93…` and actual fixed-support worker **`88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4`**. Runtime evidence is reused solely on that exact identity. The earlier 162 output/inflation/alias observations, six failures, 12 recoveries, stack capture and both complete runtime orders remain applicable. No scalar/native/resource behavior changes.

Current complete-build phase medians are **0.701823 s parsing**, **0.217373 s semantics** and **1.214269 s native emission**. Other recorded phase names and CPU/RSS data are in `matched-builds/summary.json`; catalog work overlaps parsing, and independent medians need not add.

## Where the remaining runtime losses accrue

The [19-mechanism history review and all-loss profiles](../history-and-profiles-20261119/README.md) led to two retained combinations: **63,804 → 63,756 B** from historical spill pairing plus current packet/spill advances, then **63,756 → 63,692 B** from generic impossible-guard proofs plus a fresh matcher layout. Do not attribute those gains to this parser pass.

Four new captures below bind the actual current 63,692 B worker and pinned C. They refresh the two priorities after those retained combinations; the other archived loss captures bind the prior 63,804 B reference. Each capture verifies two complete output/inflation/alias calls. Sample shares describe exclusive leaves inside each encoder tree, not elapsed phase times or forecasts. C matching is inlined into its near-optimal body, so its self bucket is not a directly comparable matcher timer. Some constructor, transfer and cleanup work is outside these trees.

- **Random prefix, 1 MiB:** 72.8% matcher, 14.2% scorer; qualified Mech/C 1.350011× / 1.351622×; larger median excess 7,128.8 µs/call.
- **Natural prefix, 4 KiB:** 23.6% matcher, 54.0% scorer; qualified Mech/C 1.055687× / 1.065612×; larger median excess 21.8 µs/call.

Prioritize the shared large-random collector path while retaining natural 4 KiB as the next-win check. The current collector's already-recorded untimed 1 MiB trace has 233.15 million original instructions, 41.02 million branches and **22.27 million stack-load instructions**. Map the remaining hot immutable-input reads, root/link checks and live ranges to source proofs before changing the compiler. The earlier literal-read-once source experiment is a lead, not current qualification; check whether those reads are still duplicated first. The secondary cursor/countdown/exit-pointer repair and early codebook unit remain available with their adverse evidence. A CLIF-only inspection retains otherwise-refused general bodies and changes the full diagnostic object; its actual collector metadata and all 6,092 target bytes match the selected body exactly (SHA `364794ef…`). Only that body can be mapped to selected offsets. The subsequent optional VCode request fails with `Unsupported feature: AArch64 block layout metadata`; its log is retained, and no VCode mapping is claimed. This small parser gain does not change the architecture priority.

## Reproduction and archive

Build this checkout using the existing recipe:

```sh
python3 rebuild.py /tmp/mech-sealed-parser --variant m1-matcher-guard-facts \
  --mimalloc-object /path/to/mimalloc.o
```

From this evidence directory, check the archived evidence or use an existing source-built compiler to reproduce normal and allocation-checked native artifacts:

```sh
python3 verify.py
python3 ../encoder-size-gap/current-matcher-guard-facts/reproduce.py \
  /path/to/compile_profile /tmp/mech-sealed-artifacts
```

The native recipe and profile files are unchanged. `variants/` preserves the original, fusion-only, allowance-only and combined continuation sources plus the test. `as-run/` records acquisition scripts and their original machine paths; they are provenance, not a portable replacement build system. Raw standalone trials, complete builds, test logs, source pins, counters and sample captures are included. Existing native bytes and full runtime evidence are SHA-bound by `reused-runtime-evidence.json`, avoiding duplicate copies. Timing varies on a fresh host.

Compiler source base: `2bc4014587fda4f31bdade02abd0cd5144d80e23` plus the archived continuation/test changes. New compiler SHA: **`bc6dfeef148b62c9576b408d761b45c5197c0caa0aec2145cd33985fc02e9104`**. Portable normal and allocation-checked replay both reproduce the exact selected object, complete metadata and bytecode. Earlier CI static architecture failures are not repaired by this parser change; no new CI-green claim is made.
