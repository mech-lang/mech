# Packet-load qualification by phase

**The qualified selection remains 63,612 B / 81 Roc / 62 libdeflate. No 63rd C win is established.** Selecting nine routine families recovers **320 B of the all-phase packet optimization's 416 B saving** while avoiding its large distribution-change penalty. The resulting word candidate is **62,844 B** and probe **63,116 B**, both with **4,736 B peak stack**. All prior peer wins observed in fourteen complete twenty-one-case discovery orders survive; full 81-case confidence qualification is outstanding.

## Implementation and experiment

Source `3fe63cb1d357006835f0715c5537ec6ad867ffbf` adds `MECH_QUALIFY_PHASE_INPUT_PACKET=only:<comma-separated exact phase names>`. Existing values such as `1` preserve the all-phase option. Empty or unknown lists make no change. The packet's initialized, aligned, nonescaping input contract is unchanged; this selects where the existing qualifier runs. No runtime guard or new proof is added.

Each selected routine includes its base, vector and no-growth optimization names. Selecting only an exported final name was an incorrect ablation setup: some names are derived after optimization, and qualifying a base without its vector counterpart prevents body sharing. The initial partial selection grows the probe to 66,460 B; its source, logs and artifacts remain archived. The corrected selection uses complete lists for each family.

The `cold` label in artifact names selects nine families: 362, 2821, 3447, 3872, 4173, 4799, 5895, 6633 and 8026. `scorer` selects family 3009. `others` selects every family except 3009, adding 475, 3956, 4879 and 4673 to the nine-family list. These names identify experiments, not a claim that a routine is cold on every workload.

The minimal compiler SHA is `4ee955a8910f7701833c49a013fb7ddf63e05d2f26035f56862fac741cc26548`. Existing modes reproduce all three artifacts for four preceding recipes; empty and unknown selections reproduce the read-proof probe exactly. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added.

## Runtime discovery

Every row has twenty-one cases, both complete orders and twenty-four balanced blocks per case. Each word/probe variant uses its matching read-proof control. The all-phase reference is measured directly against the same read-proof probe, using the exact worker whose artifacts the new compiler reproduces. Separate cohort C ratios do not establish causal differences between candidates; use each row's paired base comparison.

| Candidate | Encoder text | Natural 4 KiB / C, forward / reverse | Natural 4 KiB / matching base | Distribution change / matching base |
| --- | ---: | ---: | ---: | ---: |
| select-packet-family-probe-cold-profiled | 63,116 B | 1.016732× / 1.016522× | 1.001648× / 1.003013× | 1.004525× / 1.001098× |
| select-packet-family-probe-scorer-profiled | 63,388 B | 1.024294× / 1.027727× | 1.006227× / 1.011990× | 1.026323× / 1.028204× |
| select-packet-family-probe-others-profiled | 63,068 B | 1.014973× / 1.025385× | 0.999330× / 0.999338× | 1.061914× / 1.061228× |
| select-packet-family-word-cold-profiled | 62,844 B | 1.026291× / 1.038371× | 0.998949× / 1.003867× | 1.002024× / 1.006289× |
| select-packet-family-word-scorer-profiled | 63,116 B | 1.020033× / 1.016552× | 0.997101× / 0.989160× | 0.996512× / 0.962923× |
| select-packet-family-word-others-profiled | 62,796 B | 1.026162× / 1.030554× | 0.997729× / 0.999645× | 1.058609× / 1.061960× |
| read-window-packet-profiled | 63,020 B | 1.033081× / 1.022254× | 1.014997× / 1.003945× | 1.092648× / 1.087999× |

The direct current all-phase probe is **8.80–9.26% slower** on distribution change. Selecting just the scorer costs **2.63–2.82%** in the probe recipe. Selecting every other phase still costs **6.12–6.19%**. The narrower nine-family selection costs only **0.11–0.45%** in those probe orders and **0.20–0.63%** in the word orders, while preserving the prior observed peer wins.

The word scorer-only alternative behaves differently: natural 4 KiB takes **0.29–1.08% less time** than its matching control, with no observed previous win lost. Its distinct carry recipe and all original observations remain available. Do not transfer the probe's scorer penalty to that recipe. Combining this option with the nine-family saving is a useful next experiment.

Selecting all non-scorer phases makes the word still smaller at **62,796 B**, but it costs **5.86–6.20%** on distribution change. That candidate is retained for repair. The additional four families remove 52 B from the checked root and add 4 B to a release helper, for a net 48 B saving. The phase metadata also reports changed bodies for 4879 and 4673. Isolating the four extra selections can determine which causes the remaining penalty.

These panels establish no new full score. Small slowdowns are permitted when the individual peer win survives; no candidate is discarded solely for them. The qualified histogram remains selected.

## Complete builds on Apple M1

Fourteen balanced serial fresh-process trials per actor include source compilation and final executable linking. Compilers and fixed support are prebuilt equally; bootstrap and strip are separate. Every new Mech object, complete metadata and bytecode is hash checked. No runtime acquisition, test build, profile, counter or archive compression overlaps this cohort.

| Actor | Source through final link | Through strip | Paired build / Roc / C |
| --- | ---: | ---: | ---: |
| mech-control-reference | 2.209519 s | 2.220704 s | 0.740214× / 4.595235× |
| mech-read-probe | 2.214931 s | 2.226256 s | 0.741836× / 4.604204× |
| mech-selected-probe | 2.214145 s | 2.225238 s | 0.742105× / 4.603428× |
| mech-selected-word | 2.213414 s | 2.224574 s | 0.741357× / 4.606720× |
| mech-other-word | 2.211461 s | 2.222706 s | 0.740928× / 4.597446× |
| roc | 2.984085 s | 2.994971 s | — |
| libdeflate | 0.481338 s | 0.492381 s | — |

All Mech stripped executables remain **276,072 B**, Roc **191,912 B**, and C **155,752 B**. Peer encoder text remains **57,696 B Roc / 37,080 B C**. Phase profiles, every trial, and the separate Rust producer rebuild are archived. No compile-time gain is inferred from the code-size reduction.

## Validation and reproduction

All six new candidates pass **162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve exact recoveries**, normal/checked artifact replay and the **5,280 B stack cap**. Their measured peak is 4,736 B. The prior all-phase worker's exact artifact identity preserves its existing checks. Direct scalar native execution, error origins, ownership and finite resource accounting remain maintained. These diagnostic-entry checks do not establish ordinary managed-entry admission/recovery.

The existing read-proof test checkpoint has **25 relevant tests / 72,752 native calls**. Its proof source is unchanged; this source change only selects the existing packet qualifier. The new source compiles and passes formatting, module layout and warning policy, and is checked through six exact mode replays and the six complete artifact gates above. The full GPU suite is not claimed. The repository-wide unsafe audit still has the pre-existing missing archived `pinned_allocator.rs`.

Use a minimal compiler built from source `3fe63cb1d357006835f0715c5537ec6ad867ffbf`:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-selected-word
python3 reproduce.py /path/to/compile_profile /tmp/mech-selected-probe --role select-packet-family-probe-cold-profiled
python3 verify.py
```

The verifier checks source/artifact pins, six mode replays, error/stack/recovery gates, exact ordered graph and literal/padding layout mappings, all fourteen raw paired discovery orders and fourteen complete-build trials per actor. Initial partial-list artifacts and the filename-length failure in the combined gate helper remain archived; the corrected single-role gates all pass.

The goal remains **1× libdeflate performance**, with the next milestone **63/81 C wins**, preserving every previous individual C win and all 81 Roc wins.
