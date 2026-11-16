# Bounded lexical recognition and journal work

Combining bounded lexical recognition reuse with early release of completed cache owners reduces complete standalone parser time **11.86%**. The formatted frozen candidate repeats the result at **11.80%**, passes all **529 document tests** and preserves the complete parse bytes. It clears the original 10% screening threshold and proceeds to fresh whole-compiler PGO qualification. That qualification is separate and pending; selected compiler `8043` and its original **13.414825-second** complete executable build remain unchanged.

## What repeats

The [preceding execution census](native-parser-execution-trials.md) counted ordinary successful recognitions by source start. A fresh census now records the exact existing rule, start, consume end, context end and accepted end. Identifiers succeed 164,885 times for 15,914 exact contexts (**10.36 times each**); digit sequences succeed 124,770 times for 3,026 contexts (**41.23 times each**). Every repeated key has the same endpoint. These successful events use consume/context bounds 164,243/164,243.

Per-rule LRU simulations over successful events yield 90.35% identifier and 97.57% digit-sequence hits with 512 entries. They omit intervening other-rule attempts and admission refusals, so they are diagnostic reuse upper bounds. The actual initial prototype instead records **189,684 hits in 289,896 attempts (65.43%)**. Its cold path stores 37,622 entries; the main capture refusal is an already-resumable Unicode cursor. All raw events, simulated capacities and actual refusal counts remain in the packet.

## Architecture and measurements

The prototype derives eligible flat repeated nodes from the existing shared grammar specification. A parser-owned direct-mapped table has 1,024 slots and stores successful ASCII runs up to 64 bytes. Keys include the rule, start and both source-view bounds. A hit creates fresh node identity and the original Start/Token/Finish events, then uses the existing materialization queue. It restores the recorded logical fuel, continuation work, rewind counts and exact Unicode cursor. Conservative peak/resource checks fall back to the original execution whenever exact reuse is unproven. Unicode, long or nested nodes and unsupported contexts keep that path. No grammar or language change is made.

The first prototype enlarged the continuation from **72 to 360 bytes** and outer frame storage from **40 to 88 bytes**. A sequential CPU sample of the dispatch-only revision shows base-continuation leaves falling from 680 to 437, while `memmove` leaves rise from 515 to 778 and mutable journal descent remains large (864 to 873). These are sampled stacks, not exclusive timing or additive savings. The compact revision moves recognition state behind the existing boxed prediction storage and restores the original 72/40-byte layouts. It batches exact journal events by page. The coherent combination also releases completed sink/request journal owners before cache publication, avoiding otherwise unnecessary retained-page copying.

Each independent screen uses eight fresh serial ABBA/BAAB observations after all builds/tests finish. The timer wraps complete parsing and tree construction, excluding JSON serialization. These System-allocator, opt-level-three standalone screens are distinct from the selected PGO compiler and its approximately 1.51-second parse; no times or percentages are pooled across configurations.

| Revision | Control seconds | Candidate seconds | Change | Changes in two blocks | Units passed | Original 10% threshold |
| --- | ---: | ---: | ---: | --- | ---: | --- |
| Initial bounded lexical memo | 2.111403 | 2.102574 | -0.418% | +0.041%, -0.878% | 528 | Miss |
| Lookup at existing repeated-node dispatch | 2.091726 | 2.095665 | +0.188% | +0.097%, +0.281% | 528 | Miss |
| Compact state and batched journal appends | 2.085956 | 1.941429 | -6.929% | -6.666%, -7.189% | 529 | Miss |
| Batched memo plus completed-owner release | 2.101876 | 1.852554 | -11.862% | -11.655%, -12.068% | 529 | Pass |
| Formatted frozen combined candidate | 2.074135 | 1.829480 | -11.795% | -11.408%, -12.174% | 529 | Pass |

All forty timed full parse outputs and four diagnostic full parse outputs match every byte: **12,802,946 bytes**, SHA-256 `09fb23613e8ce330c3874ecea80ef464bad60a8f60c182a384cebf00366055ae`. This includes the complete tree, node/token identities and hashes, flags, diagnostics, statistics and restart data. The added differential tests force real hits and compare the cache-disabled parser across 3,744 combinations of existing rules, Unicode texts, bounded views, finite fuel, event limits and poll quanta. A separate journal test checks retained snapshots, partial pages, truncation and batched append. All original 526 document tests remain.

The initial and dispatch-only revisions remain adverse or insufficient results; they are not replaced by the successful combination. The three original memo revisions each miss the original promotion threshold. The combined and formatted revisions each pass it directly; their gains are measured together, never added from earlier percentages.

## Code size and exact scope

A fresh `size -m` measurement of the current 61-flag Object records **1,277,188 machine-code bytes**, **22.14 times** the pinned Roc Object's 57,696 bytes. The separate [PR #992](https://github.com/mech-lang/mech/pull/992) reports 888,196 bytes (**15.39 times Roc**) and is not integrated here. These are pre-link `__TEXT,__text` sections; the current Object's complete SHA-256 is `72926dc267c83fbfa729710fe0c51c360ae68e69a15a23858be3710627158c5f`. Earlier comparison documents used an earlier configuration. This audit does not establish a compiler-time improvement from code size alone. The current branch already contains the current PR #973 head checked by the saved audit; #991/#992 are separate branches.

The fifty-fourth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) retains every observation, source revision, exact-context event, hit/refusal count, CPU sample and raw/demangled correspondence. All 44 complete parse outputs are stored losslessly in a nested XZ archive. It also retains the first profile helper-path failure and its resume without repeating completed reference observations, and the first batched test-build visibility failure and restricted repair. Neither failure changed production source. Probe/compiler products are excluded and hash-pinned.

Fresh extraction verifies the archive and reproduces the [full summary](../../benchmarks/mech-deflate/native-profile/parser-lexical-trials-apple-m3-20261116.json) byte-identically with `python3 replay.py`. Source remains `ef5e255e817eb5bbc6df61b094a3741b31c49da2`; no production candidate is retained by this packet. Fresh PGO and complete source-to-executable qualification must establish the full-build result. Compilation below ten seconds remains open.

Follow-up: the exact six-file candidate is retained in the [compact-source qualification](native-compact-source-compile-profile.md). Same-source complete builds improve 6.379101→6.027732 seconds with identical generated code and executable images. This qualifies the supplied 243 KB source, separately from the unrun original 164 KB broad qualification plan.
