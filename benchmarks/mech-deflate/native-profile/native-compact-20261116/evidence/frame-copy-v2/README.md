# Qualify the frame-copy encoder under policy v2

The current source compiler reproduces the archived **71,192 B** frame-copy
encoder object and its timed worker exactly. The reference is 71,768 B.
The two link commands pass each candidate's own `program.o` and pin its
digest. The earlier frame-copy runtime receipt had a stale object path.
The corrected command reproduces the earlier timed worker hash.
[Actual link inputs and worker hashes](exact-bindings.json)

Both workers match all 162 output and resource observations and all six
injected allocation failures. Peak stack is **5,280 B** for each.
[Output, failure and stack checks](linked-proofs/result.json)

| Order | Reference Roc/C | Frame copies Roc/C |
| --- | ---: | ---: |
| Focused forward | 13/11 | 13/11 |
| Focused reverse | 13/11 | 13/11 |
| Full forward | 81/61 | 81/61 |
| Full reverse | 81/61 | 81/61 |

The focused screen covers all four periodic cases affected by earlier
same-worker variation. Each available order uses 96 balanced blocks per
case. Runtime v2 records **Retain**. The frame-copy recipe is selected for the matched deployment comparison.
The frame-copy encoder is **1.234× Roc**
and **1.920× libdeflate** in encoder text.
[Case outcomes](summary.json) · [Offline replay](replay-runtime.py)

The checked root contains 3,650 single-register stack loads and stores
in the reference and 3,498 in the frame-copy image. Existing-frame
adjacency can encode at most 14 additional immediate load/store pairs.
The raw encoding ceiling for that local follow-up is 56 B.
[Bound disassembly count](stack-census.json)

Use `rebuild.py --variant frame-copies --compiler-opt-level 3 --direct-source
--preselect-root-inlining --skip-refused-general-ssa` in the review tree.
