# Early codebook construction: current baseline and source boundary

**Typed source planning is implemented; early emission is not implemented.** The main architecture
task is to avoid constructing the phase's internals in controller SSA, while
preserving the qualified codebook boundary and reducing total work across all
units. `plan.md` records the hypothesis, required counters and acceptance gates.

A diagnostic build with the selected `m1-sparse-guard-rounds` compiler reproduces
the exact object, full metadata and canonical bytecode. Diagnostics are not
latency measurements. They map the qualified native boundary **4069 → 4954**
to canonical source sites **4119 → 5031**.

| Current source-level discovery | Count |
| --- | ---: |
| Global source sites | 8,223 |
| Reachable source sites | 8,078 |
| Region source sites | 885 |
| Region distinct definitions | 469 |
| Region touched typed values | 496 |
| Region touched value words | 1,578 |
| Global layout words | 16,373 |
| Conservative value-level input/output words | 44 / 8 |
| Explicit source failure sites | 44 |

The proposed region has one outside predecessor at its entry and one normal
exit. The interface remains unproved: constants, word-level liveness, ownership,
collection cursors and failure/partial-write transport require typed compiler
admission. The current late region's much narrower native interface is a
separate result, not an interface automatically available before lowering.

The current root before cleanup has **16,157 laid-out blocks and 42,255
instructions**. It allocates **831,378 SSA value slots before compaction**, then
retains 122,162. The final root snapshot has 8,750 blocks and 15,172 instructions
following the current late transformations. These counts do **not** include all
helpers and do not prove total construction work has fallen. A complete counter
set is required before claiming the early architecture removes work.

`source-cfg.json.gz`, `root-dumps.tar.gz`, `compile.log.gz` and the identity
records preserve the current input data. To replay this discovery:

```sh
gzip -dc source-cfg.json.gz > /tmp/codebook-source-cfg.json
python3 -B analyze-source-boundary.py /tmp/codebook-source-cfg.json \
  --entry 4119 --exit 5031 --output /tmp/codebook-boundary.json
```

The script parses recorded debug terminators for discovery and computes
conservative value liveness. It explicitly does not perform production
admission. `source-boundary.json` records its result and remaining unproved
requirements. No new size, build-time or runtime gain is claimed here.

## Current implementation

The new [typed preview](typed-plan/README.md) checks the actual canonical plan
before SSA, gives each operation an owner and derives dense local domains.
It separates 20 invariant constants, yields **22 input/six output words**,
verifies complete cursor/builder scopes, and passes the existing all-path
initializer. The 44/eight discovery interface above is historical.

The [all-unit construction census](construction-census/README.md) now measures
all DFG bodies and copies: the selected recipe creates **1,407,099 SSA slots,
497,878 instructions and 174,834 blocks**. The late split creates less later
traffic, but both still reach the same **831,378-slot maximum unit** and
**955,192 resident-slot peak**. These are arena positions, not register liveness
or predicted time savings.

Run `python3 -B verify-plans.py` to check both captured results and exact
artifact bindings. The [next lowering contract](lowering-contract.md) preserves
one global source ownership model across physical units. Native ABI/effect
transport and actual bounded root/helper emission remain unimplemented.

## Ordinary build validation

Nine focused planner/observer tests pass, including serialized graph imports;
the explicitly enabled preview feature type-checks. The final normal compiler
passes 162 success observations, six failure cases and 12 recoveries, and
reproduces the exact 81/62-qualified fixed-support worker. Experimental
observer/preview code is excluded from the ordinary compiler.

A dormant preview initially added a paired 4.325 ms to complete builds. Its
source is now isolated behind the `early-region-planning` build feature. The
new eight-balanced-build comparison measures **2.205288 s** through final
link, versus 2.207729 s prior, 2.977513 s Roc and 0.480597 s C. The paired
difference is **2.407 ms saved**, 7/8 faster, with a 90% interval of
**1.145–6.921 ms saved**. The initial cost and post-isolation results are both
preserved. They are a check on ordinary-build overhead, not an early-emission
gain or a sum with the prior guard scheduler cohort. Encoder text remains
63,804 B and stripped deployment 276,072 B.

`default-timing-before-gate/`, `default-timing/`, `default-source-rebuild.json`,
`default-worker-identity.json`, `validation.json`, `focused-tests.log` and
`feature-check.log` record those checks. `implementation.patch.gz` preserves
the complete source change against the recorded source base; this PR contains
the implementation in live source.
