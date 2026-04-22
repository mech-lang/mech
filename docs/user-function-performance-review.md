# User-Defined Function Call Performance Review

## Scope

This review focuses on runtime behavior for user-defined function invocation and pattern-based dispatch in the interpreter, centered on:

- `src/interpreter/src/functions.rs`
- `src/interpreter/src/patterns.rs`
- `src/interpreter/src/expressions.rs` (function-call entrypoint)

## Current call architecture (high level)

1. `expression(...)` delegates `Expression::FunctionCall` nodes to `function_call(...)`.
2. `function_call(...)` resolves user-defined functions by hashed name, evaluates call arguments, then executes the interpreted function body.
3. `execute_user_function(...)` performs argument count checks, optional matrix broadcasting, then executes either:
   - match-arm dispatch (`execute_function_match_arms(...)`) with a tail-call loop, or
   - sequential statement execution with output collection.
4. A `FunctionScope` swaps in a fresh symbol table and plan per invocation and restores caller state on drop.

This design is clean and semantically robust, but several hotspots can generate significant allocation and cloning overhead in hot call paths.

## Performance-impacting implementation details

### 1) Repeated argument evaluation/collection per call path

`function_call(...)` materializes `Vec<Value>` for user functions and again for native compiler paths. In high-frequency calls, this creates repeated allocation pressure and temporary `Value` cloning.

**Impact:** high allocation churn in dispatch-heavy workloads.

### 2) Full function-local state swap on every invocation

`FunctionScope::enter(...)` replaces symbol table + plan and restores both on drop for each call. This guarantees isolation, but creates frequent container replacement/copy activity even for tiny pure functions.

**Impact:** call overhead grows with call count, reducing throughput for fine-grained functions.

### 3) Recursive value detaching/cloning is pervasive

`detach_value(...)` and `deep_detach_value(...)` recursively clone values. They are used in argument binding, pattern matching, output collection, and enum checks.

**Impact:** expensive deep copies for nested tuples/matrices/enum payloads; amplifies with pattern-heavy code.

### 4) Match-arm dispatch is linear with repeated work

`execute_function_match_arms(...)` scans arms in source order and invokes `pattern_matches_arguments(...)` for each. For enum inputs, exhaustiveness bookkeeping can traverse all variants. No indexing/cache exists for frequent shape/kind patterns.

**Impact:** O(number_of_arms) dispatch with repeated pattern evaluation per call.

### 5) Pattern matching allocates intermediate vectors for matrix-like values

`matrix_like_values(...)` converts matrix forms into owned `Vec<Value>`, and array-pattern capture slices into newly allocated matrices (`capture_middle_matrix(...)`).

**Impact:** heavy heap traffic for large matrix patterns, even when only partial inspection is needed.

### 6) Broadcasting path recursively re-enters full user-function execution per element

`try_broadcast_user_function(...)` applies `execute_user_function(...)` per matrix element and then rebuilds output matrix.

**Impact:** multiplies full call overhead by element count; poor scaling on large matrices.

### 7) Type coercion path repeatedly invokes conversion logic

`bind_function_inputs(...)` and output coercion call `convert_to(...)` frequently, often after cloning/detaching.

**Impact:** repeated dynamic conversion checks in inner loops.

## Optimization plan

## Phase 0 — Baseline instrumentation (before behavior changes)

1. Add lightweight counters/timers around:
   - function dispatch time,
   - pattern-arm matching time,
   - detach/clone counts,
   - conversion attempts and success rate,
   - per-call allocations (sampled via allocator metrics if available).
2. Build benchmark corpus:
   - recursion-heavy scalar functions,
   - match-arm-heavy enum dispatch,
   - matrix broadcasting functions,
   - mixed polymorphic conversions.
3. Record p50/p95 latency and throughput baselines.

**Goal:** prevent regressions and identify dominant hotspots by workload.

## Phase 1 — Low-risk allocation reductions

1. Replace `&Vec<Value>` parameters with slices (`&[Value]`) where mutation/ownership is not required.
2. Reuse temporary argument buffers in dispatch paths (small-vector strategy where applicable).
3. Avoid repeated hash-map borrow/check patterns by consolidating lookups in `function_call(...)`.
4. Remove duplicated detach/conversion when value kind already matches target kind.

**Expected outcome:** immediate CPU and allocation improvements without semantic changes.

## Phase 2 — Dispatch/path specialization

1. Introduce optional fast path for small pure functions (single expression body, no local plan mutation).
2. Add arm dispatch index for common discriminants:
   - enum variant id → candidate arm list,
   - tuple arity / primitive kind quick filters.
3. Keep fallback to full linear pattern matcher for complex patterns.

**Expected outcome:** major speedup for common match-arm workloads.

## Phase 3 — Reference-aware pattern matching

1. Introduce borrowed matching APIs to avoid deep cloning in `pattern_matches_*`.
2. Only detach/clone when binding captured variables or when mutation safety requires ownership.
3. Refactor matrix-array patterns to operate on slices/views where possible.

**Expected outcome:** significantly lower memory traffic for pattern-heavy and matrix-heavy calls.

## Phase 4 — Broadcasting execution model improvements

1. Compile-and-cache a specialized element kernel for eligible broadcastable user functions.
2. Execute element kernel over matrix storage directly (typed loops), avoiding per-element full call setup.
3. Preserve semantic equivalence and fallback to current path when specialization preconditions fail.

**Expected outcome:** order-of-magnitude improvements for large broadcasted calls.

## Phase 5 — Conversion and kind-check optimization

1. Cache resolved input/output `ValueKind` per function definition.
2. Precompute conversion strategy per parameter (identity, widening, enum-check, etc.).
3. Skip dynamic conversion dispatch when strategy is identity.

**Expected outcome:** lower overhead in strongly-typed, repeatedly-called functions.

## Risk management and correctness gates

- Add invariant tests for:
  - scope isolation,
  - recursive/tail-call semantics,
  - enum exhaustiveness errors,
  - pattern-binding behavior,
  - conversion error reporting.
- Keep feature-flagged rollout for major execution-path changes (dispatch index, broadcast kernel).
- Compare trace output equivalence in debug mode before/after each phase.

## Suggested execution order

1. Phase 0 (measure)
2. Phase 1 (cheap wins)
3. Phase 5 (cheap typed-call wins)
4. Phase 2 (dispatch index)
5. Phase 3 (borrowed matcher)
6. Phase 4 (broadcast specialization)

This sequence gives early gains with minimal risk, then moves toward deeper architectural speedups.
