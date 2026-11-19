# Early codebook construction: current baseline and source boundary

**Discovery only; early emission is not implemented.** The main architecture
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
