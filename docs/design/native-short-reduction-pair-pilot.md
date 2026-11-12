# Short ordered-reduction pair pilot

The control is the reverse cursor image from `aa26a947138681dcc6a0d21434ae68b98ce248b0`. Under `MECH_PAIR_SHORT_ORDERED_REDUCTION=1`, the trial keeps the first visit and singleton exit, groups two subsequent admitted short visits, and sends a single leftover to the original scalar body. The four-lane vector prefix and scalar remainder keep their existing routes. Both costs use their original U32 wrapping arithmetic. A strict comparison selects the earlier item on a pair tie and retains the incumbent on an equal cost. The existing complete-child proof admits every read and preserves source accounting.

All eight changed bodies are the ordinary and composed ordered reductions. The linked recurring pair uses 29 instructions and no stack accesses, compared with 32 instructions for two control visits. It computes two costs and constructs one item. The linked pair still emits a second comparison for each of the two predicates. The composed bodies grow from 1,344 to 1,536 bytes and from 224- to 240-byte frames. The ordinary bodies grow from 1,504 to 1,680 bytes with the same 272-byte frame. The matcher and root Object bodies are byte-identical.

| Input | Control/Roc | Trial/Roc | Control/libdeflate | Trial/libdeflate | Trial/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Natural 8 KiB | 1.033547× | 1.052158× | 1.273626× | 1.296560× | 1.018007× |
| Natural 32 KiB | 1.146445× | 1.174160× | 1.040209× | 1.065356× | 1.024175× |
| Natural 256 KiB | 1.173303× | 1.203491× | 0.948007× | 0.972398× | 1.025729× |
| Natural 1 MiB | 1.209459× | 1.237626× | 0.971379× | 0.994002× | 1.023289× |
| Periodic 1 MiB | 0.593433× | 0.591072× | 1.004713× | 1.000716× | 0.996022× |

The nine-case screen takes **1.009291× control** across 6,912 complete timed calls. Both 1.05 gates pass on all nine cases and the peer win sets agree. All four natural sizes regress. Natural 256 KiB retires 0.35% more instructions and spends 2.37% more cycles in the separate counter cohort. Natural 8 KiB retires 0.10% fewer instructions and spends 0.62% more cycles. Its exact linked profile still places substantial work in the first visit and completion. The reduced recurring instruction count applies only to complete pairs. The trial was rejected at the targeted screen.

The two original linked workers passed 162 complete correctness checks each, including exact bytes, independent inflate, aliases, capacity, and cleanup. All measured payload metrics match. The emitted pair comparison passed 7,203 JIT oracle checks with the register allocator checker and warnings as errors, including equal costs, different incumbents, U32 wrapping, and near-limit indices. Nine exact linked profiles and the separate counter cohort completed. Full 81-case timing and decoded emission were skipped.

[Compiler patch, all timing observations, and linked machine evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-aa26a94713-pair-reduction-pilot.tar.gz). Archive SHA-256 `784deeb1e0c746afc3626bb5ec529b5c00e0644619d20b755a19351797d06b39`.
