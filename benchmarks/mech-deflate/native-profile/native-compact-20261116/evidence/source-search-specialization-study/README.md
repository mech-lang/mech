# Search-call specialization: retained source alternative

**The separate source candidate retains 81/81 Roc and 63/81 libdeflate wins in both complete 96-block orders, including every required named win.** No 64th C win is established. The smaller selected recipe remains `m1-narrow-packet-words`: 63,804 B. The eventual runtime goal remains 81/81 C.

The candidate improves several remaining losses, while increasing encoder text to **69,060 B** and complete build time to **2.497 s**. It is retained for repair, with its full source, generation commands, raw measurements and adverse results. [Results](summary.json) · [Generation recipes](recipes.json).

## Ordinary Mech source change

The original 2,719-line codec remains unchanged. The separate [2,721-line program](program.mec) changes one expression in `optimal-advance-position`: search selects the existing `bt-get-matches` helper, while skip retains `bt-advance` with its existing verified-prefix handling. The previous call supplied a dynamic `searching` flag to `bt-advance` after common setup.

This is ordinary Mech using existing functions. The search helper receives the same maximum/nice length; its result is adapted with a zero previous-match field. Cache/header writes, observation scheduling, skip counts and block decisions remain in their original caller. All 162 output/inflation/alias checks pass. No LLVM dependency or native-byte patch is added.

[generate.py](generate.py) checks the original source hash and makes exactly this replacement. Both source arms share generic optimization flags, automatically proved scalar homes and verified helper layouts. Both also include the earlier optional low-word cost carry. Their common configuration differs from the selected recipe, so a five-actor campaign measures both arms and the selected build simultaneously. This separates the source edit's effect from the composite candidate's gains.

## Both complete 81-case orders

| Order | Roc wins | C wins | Required named wins lost |
| --- | ---: | ---: | ---: |
| Forward96 | 81 | 63 | 0 |
| Reverse96 | 81 | 63 | 0 |

These are point scores under the user's selection rule. Confidence analysis is separate and has not been performed for this new source candidate.

| Full-matrix input | Candidate / selected, forward / reverse | Candidate / C, forward / reverse |
| --- | ---: | ---: |
| Natural 1 KiB | 0.982162 / 0.981835 | 1.220774 / 1.220951 |
| Natural 4 KiB | 0.995969 / 0.991605 | 0.978435 / 0.979887 |
| Random 4 KiB | **0.968045 / 0.938425** | 1.453598 / 1.476072 |
| Random 1 MiB | 1.002772 / 1.001425 | 1.286932 / 1.281615 |
| Distribution change, 16 KiB | **0.951130 / 0.946310** | 1.218358 / 1.221104 |

The composite candidate uses **3.2–6.2% less time on random 4 KiB** and **4.9–5.4% less on distribution change** in the full matrices. Large random input is essentially unchanged against the selected build. These gains do not cross another C threshold.

## Isolating the source edit

Two additional 21-case orders use all 120 permutations of five actors per case: selected Mech, original-source control, changed-source candidate, Roc and C. The original-source control uses the same configuration and source-specific proof/layout policy as the candidate. Every observed selected win survives.

| Input | Changed source / original-source control, forward / reverse |
| --- | ---: |
| Random 4 KiB | **0.962243 / 0.969310** |
| Natural 4 KiB | 1.012836 / 1.020412 |
| Random 1 MiB | **1.024116 / 1.024661** |
| Distribution change | 0.998510 / 1.008412 |
| Overlap, 320 B | 0.990603 / 0.989153 |

The helper split itself improves random 4 KiB by about 3–4%, but costs about 2.4–2.5% on random 1 MiB. The distribution gain is chiefly from the shared configuration, rather than an established gain from this source edit. Natural-4-KiB results vary by campaign: the five-actor composite was 1.2–1.7% slower than selected, while the complete matrices show a small improvement. All observations are retained; no stable natural-4-KiB speedup is inferred from that variation.

Ratios are geometric means of balanced per-block mean-time ratios. Individual timer-resolution zeros on tiny inputs remain in the raw data; every block mean is positive and no samples are filtered. Because both peer ratios use those same blocks, their quotient gives the direct paired source/control ratio.

## What the generated code explains

Fresh diagnostic counters cover the same nine declared natural/random/distribution/fragile inputs used for the preceding work census. These are pre-layout matcher counts; they include unthreaded jumps and are not final shipping instruction totals. Load/store counts describe the respective admitted kernels, with checked controller fallback outside them.

| Matcher diagnostic | Original-source control | Changed source |
| --- | ---: | ---: |
| Natural 4 KiB instructions | 1,373,811 | 1,345,693 |
| Natural 4 KiB stack loads | 105,508 | **88,099** |
| Natural 4 KiB stack stores | 30,596 | **37,810** |
| Random 1 MiB instructions | 215,081,677 | 214,248,587 |
| Random 1 MiB stack loads | 21,120,522 | 21,134,615 |
| Random 1 MiB stack stores | 6,553,945 | **7,585,407** |

The source edit reduces some natural-input spills but leaves the large-random load problem and adds about one million stores. Selecting the helper earlier has not removed the state carried through the outer collector loop. Keep the smaller-input gains and repair that lifetime/store cost; this is not evidence of an architectural ceiling.

The existing `MECH_PRIVATE_VECTOR_PROVE_UNSIGNED_EXTREMA` alternative was also revisited on the current compiler/carry foundation. Enabled and disabled emission produce identical object, metadata and bytecode. No duplicate latency campaign was run for identical code.

## Complete build cost on Apple M1

| Actor | Encoder text | Source through final executable link | Stripped executable |
| --- | ---: | ---: | ---: |
| Selected Mech | **63,804 B** | **2.219133 s** | 276,072 B |
| Original-source control | 67,268 B | 2.212468 s | 276,056 B |
| Changed-source candidate | **69,060 B** | **2.497319 s** | 276,056 B |
| Roc | 57,696 B | 2.988252 s | 191,912 B |
| libdeflate C | 37,080 B | 0.481795 s | 155,752 B |

Ten balanced serial fresh-process trials per actor include final linking. Compilers and fixed support are prebuilt equally; compiler bootstrap and stripping are separate. Every generated Mech object, full metadata and bytecode is checked against the measured recipe. Every stripped executable matches its own warmup identity.

Selected paired build ratios are **0.743219× Roc / 4.601969× C**. Candidate ratios are **0.835627× Roc / 5.153037× C**, and **1.123729× selected**. Candidate text is **1.862× C**. Peak measured stack is 4,736 B selected, 4,576 B source control and 4,864 B candidate, within the 5,280 B cap.

| Compiler phase median | Selected | Original-source control | Changed source |
| --- | ---: | ---: | ---: |
| Parsing | 703.3 ms | 703.5 ms | 704.5 ms |
| Semantic lowering | 217.5 ms | 217.1 ms | **404.7 ms** |
| Native pipeline | 1,225.6 ms | 1,219.5 ms | 1,302.2 ms |

The source change activates 838 additional canonical actions and 85 lexical blocks. Most added build time is in semantic lowering; the parser barely changes. Separate phase medians do not sum exactly. The extra analysis/code cost also needs repair.

## Checked behavior and reproduction

Each arm passes 162 successes with exact bytes, inflation, borrowed aliases and cleanup; six final-allocation failure actions; 12 injected failures with exact recoveries; measured stack; and normal/checked register-allocation emission. Four actual replays using the public source/profile paths reproduce all three native artifacts exactly.

Changing the source renumbers later instructions. The original same-source gate initially rejected the differing origin arrays. The source-aware check preserves status and maps only the named allocation actions in the unchanged prefix/suffix: for example, final output allocation 7164 becomes 8002, with the same node/local action. The original default policy is unchanged. A copied control's stale link receipt was also repaired by relinking at its staged path and proving the entire worker hash unchanged. Both initial failures and successful checks are archived.

On matching Apple AArch64 hardware with a compiler built from this branch:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-source-split --role source-split
python3 reproduce.py /path/to/compile_profile /tmp/mech-source-split-checked --role source-split --checked
python3 verify.py
```

`--role source-control` rebuilds the separate original-source control. The portable verifier checks archive/member identities, source generation, actual native objects, all saved runtime observations, every required full-matrix win, and the ten complete-build orders. [Verification receipt](verification.json).

The selected default remains unchanged. The next work is to repair collector state/lifetimes and the large-random store cost while retaining the smaller-input gains, then measure setup/mapping/retirement on the closest natural-1-KiB loss. Mech frequency capture, identical-input phase replay, and the size/build targets also remain open. **64 C wins is next; 81 remains the goal.**
