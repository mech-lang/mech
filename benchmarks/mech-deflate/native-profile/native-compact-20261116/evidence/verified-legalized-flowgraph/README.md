# Reuse the verified CFG after legalization

The Cranelift verifier constructs a fresh control-flow graph and dominator tree
from legalized IR. Optimization takes ownership of those results for the
unreachable-code and constant-phi passes. Standalone legalization still
invalidates its caches. The path with verification disabled builds both
graphs normally. The old and new libraries agree on optimized IR, verified
analysis, invalidation, and the invalid return diagnostic.
[Library checks](checks/flowgraph-verification.json)

| Matched Apple M1 | Control | Graph reuse |
| --- | ---: | ---: |
| Native emission | 1.52283 s | 1.50987 s |
| Source through final link | 3.85846 s | 3.84972 s |
| Compiler peak RSS | 401,506,304 B | 401,448,960 B |
| Encoder text | 71,768 B | 71,768 B |

The paired median native reduction is **0.788%**
with 8 of eight faster native phases. Complete
build time is **0.205%**
lower in the paired median, with 7 faster
complete builds. The prototype misses the declared timing gate and remains a recorded screen. The run uses the prebuilt compilers and
fixed runtimes and link adapters for both Mech builds and the peers.
[Predeclared timing gate](matched/plan.json) · [Raw build trials](matched/timing.json.gz)

Both compilers emit the exact same bytecode, encoder object, complete
metadata and executable. The relinked benchmark worker reproduces the
recorded identity and passes 162 exact calls. Its two full runtime orders
record **81/81 Roc and 61/81 libdeflate**.
[Complete build and artifact report](report.json) · [Original runtime receipt](../bound-71768-runtime/README.md)
