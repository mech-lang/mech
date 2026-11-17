# Checked-root compiler work

Both studies use the retained 71,768 B encoder, including the 51,240 B checked root. The source CFG dump was acquired with the exact 1,034,027 B bytecode, 159,384 B object and complete metadata. The inventory checks the dump against the original metadata by source origin and independently reconstructs the same 8,078 reachable sites out of 8,223. The 5,690 Plan values and every digest are recorded with the results. The archived fixed-link benchmark worker passed 162 exact calls and recorded 81/81 wins against Roc and 61/81 against libdeflate. [Input receipt](source-cfg-compile.json) · [Inventory and hashes](inventory.json) · [Runtime](../bound-71768-runtime/README.md)

## Scalar transport before SSA

The aggregate emitter snapshots the source words for a `Pack`, `Copy` or Tail carrier before filling the destination. An SSA word read calls `use_var`, so skipping a read at this point could avoid SSA resolution and phi construction. The previous captures place checked-root SSA self time at 188–191 ms and the enclosing cleanup span at 590–618 ms. [Aggregate emitter](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/hosts/gpu/src/native_artifact/codegen/common/aggregates/mod.rs#L80-L143) · [Native timings](../native-exclusive-work/README.md)

| Reachable transport | Sites | Field positions | All-scalar product sites | All-scalar positions | Untagged mixed product sites | Plain scalar siblings |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Pack | 325 | 4,392 | 32 | 106 | 264 | 776 |
| Copy | 1,086 | 4,711 | 35 | 111 | 153 | 574 |
| Tail seed | 56 | 1,126 | 0 | 0 | 54 | 218 |
| Tail latch | 56 | 1,126 | 0 | 0 | 54 | 218 |
| Total | 1,523 | 11,355 | 67 | 217 | 525 | 1,786 |

Fields include packed constants. An all-scalar product contains only one-word scalar leaves and has no owner lanes. A mixed product has at least one plain scalar leaf as a sibling of a protected field. The census stops at tags, arrays and other protected layouts. Another 752 copies are single scalars and 179 transport sites have protected shapes without a plain scalar sibling. Five of the single-scalar copies lie on synthetic boundaries. [Shape and decline counts](inventory.json)

The static demand model propagates a destination's live lanes back to the matching sources of a parallel copy. It preserves all non-scalar lanes, every reported ownership effect, source order and the complete operands of unsupported actions. Project and Observe demand their selected fields and guards. The model demands all tracked fields at the reported numeric-phase boundaries and private handoffs, private indexed chains, packed-word intervals, widened reductions, collection actions, and explicit checked failures. There are 1,088 such sites in the dump. All saved results reached a fixed point. [Census implementation](inventory.py) · [Emitter's synthetic predecessors](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/hosts/gpu/src/native_artifact/program/compact/emit.rs#L633-L669)

| Demand model | Sites with removable source reads | Source reads | Tail reads |
| --- | ---: | ---: | ---: |
| Strict all-scalar products | 0 | 0 | 0 |
| Strict with single-scalar copies | 0 | 0 | 0 |
| Protected siblings in mixed products | 7 | 14 | 0 |
| All-scalar products with source edges alone | 1 | 3 | 0 |
| Mixed products with source edges alone | 12 | 25 | 0 |

The seven mixed sites each have two candidate scalar siblings and both a planned owner transfer and a retirement. The sensitivity counts these scalar operands while keeping the protected siblings live. Modifying those sites requires an ownership and synthetic-path proof in the emitter. The source-edge-only rows remove opaque-route and failure barriers to estimate coverage of this model. The three all-scalar reads at site 6982 come from that sensitivity. These are source counts and the corresponding phi and instruction reductions have yet to be measured. [Per-site results](inventory.json)

### Causal plan and decision

1. Preserve the exact checked root, its ordinary and synthetic routes, and the reported worker identity.
2. Count source transport by physical field and match its reachability and source origins to the exact retained metadata.
3. Prove demand to bounded convergence with conservative whole-field behavior for unsupported effects, checked failures, ownership, and any still-opaque bypass.
4. If a material slice is admitted, pass the demanded scalar ordinals to the checked-root SSA transport and filter before `words.read`. Preserve parallel snapshots and the existing storage behavior for all protected lanes.
5. Compare the same code with the mask forced to all fields. Count omitted reads and constants, actual phi creation, before/after instruction counts and the propagation cost.
6. Verify ABI, origins, failure and refusal behavior, ownership and runtime results. Compare canonical bytecode, object, complete metadata and the fixed-link worker. Reconcile any construction-count differences explicitly and qualify changed target bytes as a separate candidate.
7. Park at the inventory. Reopen when a source-backed census identifies a materially larger checked-root transport family, or when construction counters isolate a substantial phi or memory effect from a small family. Cover its guard-decline, failure, ownership and synthetic predecessors before suppressing reads.

## CFG and dominator reuse

The retained checked root performs `share_repeated_bounds`, `reuse_dominating_window_facts`, `reuse_selected_read_bounds`, `prove_scalar_spans` and `prove_packed_bases` consecutively. Both logs record 0, 6, 145, 0 and 0 changes respectively. Each wrapper unconditionally resolves aliases and starts with a full CFG and dominator build. A nonzero change triggers a new pair and the existing unreachable, phi and verification work. [Root sequence](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/hosts/gpu/src/native_artifact/program/compact/emit.rs#L1533-L1556) · [Range wrappers](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup/mod.rs#L495-L675)

| Proposed reused entry pair | Preceding pass | Old trace | Shared trace |
| --- | --- | ---: | ---: |
| Dominating window facts | Repeated bounds, 0 edits | 0.808 ms | 0.808 ms |
| Packed bases | Checked scalar spans, 0 edits | 0.804 ms | 0.799 ms |
| Selected read bounds | Dominating windows, refreshed and pruned after 6 edits | 0.812 ms | 0.820 ms |
| Checked scalar spans | Selected reads, refreshed and pruned after 145 edits | 0.806 ms | 0.807 ms |
| All four pairs | | 3.231 ms | 3.235 ms |

The first two pairs are redundant across graph-neutral boundaries. For the last two, rebuild after the modifying pass as today. Unreachable elimination only removes blocks outside entry reachability and updates their CFG edges in place. The existing dominator tree skips unreachable predecessors and caches DFS order based on the surviving ordered successors. Constant-phi removal and alias resolution only change operands, including branch arguments. These operations keep the branch instruction IDs, ordered reachable successor stream and surviving layout required by the next range proof. [Unreachable elimination](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/vendor/cranelift-codegen/src/unreachable_code.rs#L17-L72) · [Phi removal](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/vendor/cranelift-codegen/src/remove_constant_phis.rs#L335-L402) · [Dominator order](https://github.com/mech-lang/mech/blob/d3c120366916c9921ed64a6840bb7a8feba791af/vendor/cranelift-codegen/src/dominator_tree/mod.rs#L339-L443)

A future local experiment would pair a fresh initial analysis with explicit range-pass effects, preserve every verifier call, and compare reuse enabled and disabled. Any unexpected block or terminator edit, successor reorder, or layout change requires fresh graph and dominance. The `is_valid` flags only signal prior computation. The replay identifies the range block by the unique direct-child timing sequence bracketed by checked-add and equal-join work, and counts only the listed entry graph pairs. It uses already acquired elapsed spans. The shared trace attributes 342.803 ms to all verifier, CFG and dominator pass self time within coordinator cleanup. The 3.235 ms range slice is 0.94% of that budget and does not include any verifier work. Park the slice. Reopen when pass-labeled traces locate materially more repeated full analysis work across boundaries with the same ordered layout and successor guarantees. [Trace replay](cfg-chain.py) · [Trace event indices](cfg-chain.json) · [Prior verification work](../verified-legalized-flowgraph/README.md)

## Replay

Run from this directory with the archived native profile in the adjacent evidence directory.

```sh
python3 -B inventory.py --source-cfg source-cfg.json.gz --metadata ../native-exclusive-work/old/metadata.json.gz > /tmp/scalar-transport-inventory.json
cmp inventory.json /tmp/scalar-transport-inventory.json
python3 -B cfg-chain.py ../native-exclusive-work > /tmp/cfg-chain-replay.json
cmp cfg-chain.json /tmp/cfg-chain-replay.json
```

The archived diagnosis used no new compiler builds or timing acquisitions. The census and trace replay changed no compiler source.
