# History review and every remaining C-loss profile

**Prioritize the match collector on large random input, while keeping natural 4 KiB as the next-win check.** The audit reference is **63,804 B and 81/81 Roc / 62/81 libdeflate in both complete orders**. Its follow-up [current spill-pair retest](../encoder-size-gap/current-register-spill-pairs/README.md) now qualifies a **63,756 B** recipe with every prior peer win held. The profiles below continue to bind the exact audit reference; they are not transferred to the changed worker.

The review covers **19 historical mechanisms** and **36 fresh encoder captures**: Mech and C on all **18 distinct contracts representing the 19 remaining named C losses**. Every captured worker is the exact selected Mech `5afce59a…` or pinned C `06e3b4b1…`. Original Mech source and peer executables are unchanged. Each capture also verifies two complete output/inflation/alias calls. One initially idle C capture was preserved and reacquired with a longer command stream; all final captures contain at least 1,896 encoder-tree samples.

## Where the losses accumulate

The gaps below come from the existing qualified paired campaign. The profiles are new, separate diagnostics. Percentages are exclusive leaf samples inside the Mech encoder function tree.

| Case | Qualified Mech/C, forward / reverse | Larger median excess per call | Matcher samples | Scorer samples |
| --- | ---: | ---: | ---: | ---: |
| Random prefix, 1 MiB | 1.3754× / 1.3771× | **7.652 ms** | **72.5%** | 13.2% |
| Random prefix, 256 KiB | 1.3905× / 1.3946× | **1.983 ms** | **71.5%** | 14.6% |
| Random prefix, 128 KiB | 1.4051× / 1.4051× | **1.002 ms** | **70.6%** | 15.5% |
| Distribution change, 16 KiB | 1.3792× / 1.3784× | 182.9 µs | 33.3% | 31.0% |
| Natural prefix, 4 KiB | 1.0496× / 1.0728× | 25.8 µs | 23.8% | **54.2%** |
| Natural prefix, 1 KiB | 1.3441× / 1.3446× | 20.3 µs | 14.7% | 34.0% |

The 4 KiB natural case still has the smallest proportional gap, requiring about 6.8% less time. Large random cases need approximately 27–29%, but expose a shared expensive collector path across multiple losses. The random-prefix excess scales approximately linearly across the recorded sizes: a descriptive fit has **7.264 ns/byte**, a **36.95 µs** intercept and R² 0.99993. This describes those medians; it does not identify a cause or predict an optimization's gain.

The full [case ranking and profile matrix](summary.json) includes every loss, reported allocation/copy/zeroing counters, both actors' functions and explicit capture boundaries. C's near-optimal body includes its inlined matching code, so its self samples are not a directly comparable standalone matcher timer. Constructor, final-transfer and cleanup work outside the selected function trees is not all included. **These sample shares are not elapsed phase times; do not multiply them by the compression medians to forecast savings.**

## The current collector still does substantial work

A separate counter clone executes the original collector instructions and verifies outputs. It is never used for latency.

| Current collector | Calls | Executed original instructions | Branch instructions | Stack-load instructions | Stack-store instructions |
| --- | ---: | ---: | ---: | ---: | ---: |
| Random prefix, 1 MiB | 35 | **245,026,411** | **45,024,643** | **25,947,639** | **9,698,800** |
| Random prefix, 4 KiB | 1 | 824,100 | 145,835 | 94,115 | 33,128 |
| Natural prefix, 4 KiB | 1 | 1,520,054 | 281,929 | 143,044 | 39,683 |

These counts cover `mech_compact_numeric_475_vector`, whose current body is 6,196 bytes before final extent padding. They do not count C instructions, and they do not prove that every stack access can be removed. The retained earlier spill-forwarding result already reduced natural 4 KiB stores to 39,683; that gain is present here.

The large-random sample's collapsed collector frame lists offsets **1292 and 1420** among its sampled PCs. Their surrounding code contains current/linked-word loads, range/high-bit checks, root/link setup and a stack reload. The collapsed sample does not supply an individual frequency for each listed PC. The [full current disassembly and dynamic counts](matcher-counts/) support tracing the remaining checks and live ranges before changing the compiler.

Allocation requests are large—26.4 MB requested and 6.6 MB reported zeroed for 1 MiB random input—but zeroing is only 0.04% of that encoder sample tree. Volume alone is insufficient reason to choose allocation work ahead of the measured collector hotspot. Stored output already uses source `sequence/append-sublist`; its copy samples are small in this capture.

## What the history changes

[The complete 19-entry review](history.json) links every claim to its recorded source/evidence and distinguishes work already adopted from candidates to repair.

- **Allocator spill pairing is the first historical retest.** It reduced old 1 MiB matcher instructions from 254.8 million to 246.4 million and preserved every old full-suite peer win. The old policy rejected roughly 0.6% slowdowns on two still-winning cases. The live flag `MECH_COMPACT_PAIR_REGISTER_SPILLS` is absent from the selected recipe. Re-evaluate it with current scoped packets, spill forwarding and the allocation checker; reconstruct valid profiles before measuring. The old aligned variant also held peer wins but needs a current stack check and modern output receipts.
- **Dead spill stores and affinity:** the old 16-byte dead-store saving was left untimed. Its dynamic execution count matters more than its static size; inspect the current hot stores before deciding. By contrast, the affinity prototype made zero preferred assignments, so rerunning it needs evidence that the rule can now fire.
- **Old cursor work:** it preserved every focused peer win but v1 rejected the empty-input slowdown. That alone is not a rejection under today's policy. Its old 43,020 repeated scorer-bound reloads were subsequently addressed by the selected admitted-cost-length proof. Revisit the old countdown and discarded-exit-pointer recipe with that proof and current vector-word hoisting. Root register preference is a separate change, not the explanation for the callee reload repair.
- **Caller-prepared addresses:** the old 71,048-byte alternative held its old peer wins but exceeded the stack cap by 16 bytes. Current selected stack is 4,848 bytes, leaving 432 bytes of headroom. This makes a current-frame retest reasonable; it does not prove the old packet extension will cost the same amount today. Check current equivalent lookup preparation before porting the old mechanism.
- **Short scalar pairs:** the old paired arm introduced spills and text growth. Reconsider it only with the current bound/liveness improvements and a current scalar trace. Keep independent controls rather than rerunning a threshold grid.
- **Literal reuse:** an older ordinary-source experiment reused the current literal for observation sampling and the cache header, improving random 4 KiB by 1.696% in a substantially different generic configuration. Inspect whether the current machine code already merges those reads. Prefer a generic immutable-load/value-reuse proof on unchanged Mech source; the old source result is not current qualification.
- **Stale-root batching:** the old source rewrite grew text by 13,648 bytes, slowed 1 MiB random by 17.505%, lost peer wins and exceeded the production parser allowance. Those are substantive failures. A retry needs compatible generic admission and less constructed/emitted work; copying the rewrite unchanged would not address them.
- **Already adopted:** admitted scorer cost length, dense-scorer selection and its direct-branch recovery, shared growth refusal, matcher spill forwarding, matcher fallthroughs, direct scalar homes and preserved root registers. Extend identified remaining costs rather than report these old savings again.

The early codebook unit remains a compiler/size architecture experiment: it lowers total constructed SSA but currently grows the artifact, exceeds stack and costs build time. Keep repairing its boundary/planning costs. Its smaller-root counts do not establish progress on the collector's runtime loss.

## Next action and acceptance

First retest the live allocator spill-pairing feature against the selected recipe, using the symbolic allocation checker and profiles rebound to the actual paired bodies. Then trace the collector's hot input/root/link checks and remaining spill lifetimes on large random input. Compare that with historical literal reuse, current scoped packets, existing spill forwarding and source range/ownership facts. Choose one generic proof or live-range reduction from that evidence. Current preserved-register preference applies to eligible checked roots, so it should not be assumed to optimize this numeric leaf.

Keep natural 4 KiB as a next-win canary and retain the cursor/countdown/exit-pointer repair as a bounded secondary experiment. A collector improvement can be checked across the random family and the natural cases. The larger gap is acceptable as an investigation target because the path covers several losses and a much larger amount of elapsed work.

Any changed worker still needs exact outputs/aliases/errors/recovery, stack at most 5,280 B, both complete runtime orders, every existing peer win and at least 81 Roc / 62 C. Small slowdowns without peer losses are allowed. Record complete encoder text and matched fresh-process source-through-final-link time before selecting a result. The goal remains 1× libdeflate code and build time, with more runtime wins; this review introduces no new build-time or code-size claim.

## Replay

```sh
python3 analyze.py
python3 verify.py
```

Analysis reads the archived profile data and the qualified case ranking from the parent evidence package. Pinned actor binaries and Mech metadata are included for fresh captures; `as-run/` records acquisition, the idle-capture retry, counter instrumentation and history-review inputs. The analyses do not require the original temporary paths. Capture fractions vary on a new run; the archived SHA bindings, counters and acceptance boundaries can be checked exactly.
