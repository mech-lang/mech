# Selected computed matrix extents

Base: ac7f16c8aa4477783251a44ecbb0fe47267b19d1 (#960).

## Required source operation

The pinned package at 671077457ff7ef3a3d44caf715a3fc595addf379 uses conditional `packed.concat(List.repeat(0, n - len))` versus `packed` in BlockOut.roc:337–345, reached from the sequence-writer table assembly at lines 750–752. The permanent connected block witness first failed at this operation with `source-semantics/incompatible-match-result-kind`.

Minimal acceptance source:

```mech
choose(values<[u64]>, padding<[u64]>, flag<bool>) => (out<[u64]>) :=
  out := (flag? | true => [values padding] | false => values).
choose(values<[u64]>, padding<[u64]>, flag<bool>)
```

## Design

Only otherwise-incompatible pure selected matrix results with computed, unconstrained turn-local dimensions reify those computed extents as free result axes. Already-matching result schemas get no added conversion calls. Constants, repeated identical computed axes, scalar element kinds, constrained/persistent dimensions, nested element schemas, activation bodies, and tail-recursion specialization retain their existing rules. The source emits the existing canonical `convert/kind` operation. Artifact validation and codec continue requiring exact branch schemas; no validation rule or executable revision is weakened.

The resident conversion binder first uses the existing symbolic cast plan. Its narrow fallback requires snapshot matrix carriers and exactly equal closed source and destination schemas at the current binding. The executor validates the current target shape and preflights existing resource limits before rebinding the immutable snapshot. Review found that this path currently rebuilds the matrix payload for differing symbolic schemas. Backing-storage preservation remains an unresolved finding; no zero-copy claim is supported yet. Source and destination schema keys are checked before publication. Scalar and dense conversion paths remain unchanged. Native execution uses this maintained resident library binding; there is no new native-only semantics or interpreter fallback.

## Validation and limitations

Receipts record exact commands and feature profiles. The selected-matrix host tests cover original and decoded artifacts, both arm orders, empty and nonempty buffers, selected division failure, unselected failing paths, resident/native agreement, unchanged prior publication and input aliases, recovery, cleanup, and fixed-geometry/unlike-element rejection. Unit tests retain shared dimensions and reject generalization of constant, bounded and persistent dimensions.

An existing conversion unit test, `present_option_preflights_payload_budget_before_publishing`, fails with required output bytes 16,777,560 while its assertion expects 16,777,568. The identical failure was reproduced after restoring **all tracked engine source changes to the exact parent**, verified by `git diff --exit-code -- src/engine`. The new unreferenced helper file and host fixture work were not compiled by that parent `mech-engine --lib` command. The inherited test's expectation is unchanged. It is explicitly excluded from the focused passing receipt, not disabled in source or CI, and remains separate baseline work.

The initial new test used an insufficient 1 MiB account and was denied at 1,267,113 bytes. The maintained acceptance uses the existing 128 MiB component-test account. Compiler defaults and existing test allowances were not changed.

The connected block qualification additionally reached 5,322 native control blocks, exceeding the default 4,096 compile setting. It uses the existing full-component 16,384 setting. This source integration is separate work; it does not establish a complete compressor or benchmark.

All PRs remain draft pending green CI and clean Codex review. Remote CI is not awaited under the user's existing instruction.

Passing receipts: `selected-matrix-review-host` runs 4 maintained host tests (352 filtered, 6.15 seconds test execution; 30.49 seconds including build). `selected-matrix-runtime-qualified` runs 9 unit tests (1093 filtered, including the explicitly excluded inherited failure; 0.03 seconds test execution, 65.83 seconds including build). The passing unit receipt predates the final frontend narrowing to only mismatched result schemas; its tested normalizer and conversion code are unchanged. The final host receipt covers that narrowing and proves already-matching results introduce zero MatrixConversion calls. Total: 13 unique compiler tests. The connected block is a separate component qualification, not included in that count.

## Review follow-up: matching initial extents

The constant-offset issue is reproduced with `row := [x | x <- values<[u64]>]` followed by a selection between `[row 0u64]` and `row`. The review’s unshaped input example instead fails the existing fixed-row/general-matrix geometry check, so it is not the binding reproducer. The corrected example reached resident `KernelBind/UnsupportedLayout` on the published head.

Binding now derives the target initial shape from the source extents before checking closed-schema equality. Live shape resolution and resource preflight remain unchanged. Five selected-matrix host tests pass (original/decoded artifacts, resident/native agreement included), 353 filtered, 6.26 seconds execution, 114.67 seconds including rebuild. The existing four tests retain their assertions and allowances. The storage-copy finding remains open, including preservation of schema-sensitive receipt tokens and shape ownership accounting.
