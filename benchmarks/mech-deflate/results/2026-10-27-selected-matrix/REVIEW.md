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

The resident conversion binder first uses the existing symbolic cast plan. Its narrow fallback requires snapshot matrix carriers and exactly equal closed source and destination schemas at the current binding. The executor validates the current target shape and preflights existing resource limits before rebinding the immutable snapshot. The storage follow-up below shares immutable primitive matrix payloads after proving equal closed geometry, while retaining independently validated destination metadata. Source and destination schema keys are checked before publication. Scalar and dense conversion paths remain unchanged. Native execution uses this maintained resident library binding; there is no new native-only semantics or interpreter fallback.

## Validation and limitations

Receipts record exact commands and feature profiles. The selected-matrix host tests cover original and decoded artifacts, both arm orders, empty and nonempty buffers, selected division failure, unselected failing paths, resident/native agreement, unchanged prior publication and input aliases, recovery, cleanup, and fixed-geometry/unlike-element rejection. Unit tests retain shared dimensions and reject generalization of constant, bounded and persistent dimensions.

An existing conversion unit test, `present_option_preflights_payload_budget_before_publishing`, fails with required output bytes 16,777,560 while its assertion expects 16,777,568. The identical failure was reproduced after restoring **all tracked engine source changes to the exact parent**, verified by `git diff --exit-code -- src/engine`. The new unreferenced helper file and host fixture work were not compiled by that parent `mech-engine --lib` command. The inherited test's expectation is unchanged. It is explicitly excluded from the focused passing receipt, not disabled in source or CI, and remains separate baseline work.

The initial new test used an insufficient 1 MiB account and was denied at 1,267,113 bytes. The maintained acceptance uses the existing 128 MiB component-test account. Compiler defaults and existing test allowances were not changed.

The connected block qualification additionally reached 5,322 native control blocks, exceeding the default 4,096 compile setting. It uses the existing full-component 16,384 setting. This source integration is separate work; it does not establish a complete compressor or benchmark.

All PRs remain draft pending green CI and clean Codex review. Remote CI is not awaited under the user's existing instruction.

Passing receipts: `selected-matrix-review-host` runs 4 maintained host tests (352 filtered, 6.15 seconds test execution; 30.49 seconds including build). `selected-matrix-runtime-qualified` runs 9 unit tests (1093 filtered, including the explicitly excluded inherited failure; 0.03 seconds test execution, 65.83 seconds including build). The passing unit receipt predates the final frontend narrowing to only mismatched result schemas; its tested normalizer and conversion code are unchanged. The final host receipt covers that narrowing and proves already-matching results introduce zero MatrixConversion calls. Total: 13 unique compiler tests. The connected block is a separate component qualification, not included in that count.

## Review follow-up: matching initial extents

The constant-offset issue is reproduced with `row := [x | x <- values<[u64]>]` followed by a selection between `[row 0u64]` and `row`. The review’s unshaped input example instead fails the existing fixed-row/general-matrix geometry check, so it is not the binding reproducer. The corrected example reached resident `KernelBind/UnsupportedLayout` on the published head.

Binding now derives the target initial shape from the source extents before checking closed-schema equality. Live shape resolution and resource preflight remain unchanged. Five selected-matrix host tests pass (original/decoded artifacts, resident/native agreement included), 353 filtered, 6.26 seconds execution, 114.67 seconds including rebuild. The existing four tests retain their assertions and allowances. The storage-copy finding is addressed by the follow-up below, including schema-sensitive receipt tokens and shape ownership accounting.

## Review follow-up: immutable matrix storage

Equal closed primitive matrix schemas now retain the same immutable matrix data. Destination parameter values are validated against the destination schema and receive their own admitted shape owner. Nested element schemas and differing geometry retain the rebuilding/rejection paths.

Two associated invariants needed explicit handling: receipt tokens include schema/shape metadata, and budget registrations formerly identified only payload plus schema-table owner. The bounded receipt cache now stores its metadata prefix alongside the completed token; a different prefix computes the exact original algorithm instead of returning a stale token. Import reuse additionally requires the same shape owner. Memory witnesses include both the original payload shape and the rebound shape when they differ. No payload element materialization occurs on the admitted rebind path.

The regression fails the backing-storage identity assertion on 125502c40884ab69a3a38b5b9f75dc2cfc524890 before the storage change. It checks shared storage, unchanged contents, exact independent receipt computations for both metadata identities, wrong-extent rejection, already-budgeted inputs, insufficient-reservation rejection without lost publication/charge, retry, repeated import reuse, and final cleanup. The final core rerun passes all 337 tests, including the strengthened already-budgeted-input regression (0.84 seconds execution, 14.80 seconds including build). Static architecture/catalog/inventory/formatting checks pass. Five selected-matrix host tests and two separate connected-block component tests pass (7 total, 351 filtered, 127.64 seconds execution, 616.64 seconds including rebuild). Original/decoded outputs and independent decompression agree. The connected compressor fixtures remain separate uncommitted integration work, not part of this compiler patch.

The storage fix adds bounded metadata: one extra u64 in the inline receipt cache and one shape-owner pointer per budget registration. In the two-block sequence witness, allocations decrease from 2,800,039 to 2,799,015 while cumulative allocated bytes increase from 248,650,194 to 249,172,444; both are process allocation counters, not peak memory or payload-copy counts. Timing remains approximately 2.06–2.56 seconds. This is a storage-correctness fix with explicit sharing evidence, not a demonstrated full-compressor optimization.

The final conversion/shape unit run executes all 10 selected tests without skips: nine pass and the one previously proven inherited option-budget expectation fails with the identical required 16,777,560 versus expected 16,777,568 (limit 16,777,216). Its expectation and source remain unchanged. The final static check also passes. Combined unique passing compiler tests: 337 core + 5 host + 9 engine = 351; the two connected-block tests are additional component evidence.
