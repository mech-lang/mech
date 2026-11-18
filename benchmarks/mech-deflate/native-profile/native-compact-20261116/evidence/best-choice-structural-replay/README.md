# BestChoice winner replay

Four balanced Apple M1 pairs compare the ordinary optimized parser probe with
a separate, timed attribution build. Both produce the exact identity-bearing
tree, 718,353 events and 3,967,302 fuel charges. The one-shot parser medians
are 1.880294 s and 1.884513 s. A separate count-only build checks trial effects
and the final execution. [Frozen methods](PLAN.md) · [All observations](report.json)

| Observed phase | Exclusive elapsed time |
| --- | ---: |
| Grammar attribution window | 1.843 s |
| Winner re-execution in BestChoice | 1.142 s |
| Statement variable-definition winner, exclusive | 0.643 s |

The counted variable-definition family has 3,128 clean replays. The provisional
criterion admits 1,588 shapes, covering 116,615,736 of 231,376,278 logical
work units in the family. Logical work is a coverage measure. The prototype
will test whether the corresponding elapsed time can be removed after the
cost of recording, admission checks, and replay. It must decide admission
from the retained trial summary and live state before replay mutates anything.
[Separate count-only output](count-runs/census.json)

The measured source is dae46c6867. The later journal directory move preserves
the module bytes. This evidence measures the existing parser's diagnostic
probe. The structural replay implementation and its differential checks are
in progress.
