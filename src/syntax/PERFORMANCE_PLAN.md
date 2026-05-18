# Syntax Parser Performance Audit and Fix Plan

## Scope

This document reviews parser hot paths under `src/syntax/src` and proposes concrete, staged fixes.

## Key Performance Issues

### 1) `ParseString` cloning is expensive and happens constantly

`ParseString` derives `Clone` and carries `error_log: Vec<(SourceRange, ParseErrorDetail)>`. Every clone duplicates this vector. The parser frequently clones input for speculative parsing (`expression`, `factor`, `is`, `is_not`), so cloning cost can scale with both input length and number of parse attempts.

Relevant code:
- `ParseString` definition and `Clone` derive (`src/syntax/src/lib.rs`)
- Repeated `input.clone()` fallback chains in `expression` and `factor` (`src/syntax/src/expressions.rs`)
- `is` / `is_not` cloning (`src/syntax/src/parser.rs`)

**Impact:** high CPU and allocation overhead in ambiguous grammars and deeply nested expressions.

**Fix:**
- Move error logging out of `ParseString` into shared state (`Rc<RefCell<Vec<...>>>`) or an external parse context.
- Keep `ParseString` as a tiny cursor/view object so clone is trivial.
- Replace manual fallback chains with `nom::branch::alt` + `cut` where possible to reduce speculative retries.

---

### 2) Terminal/tag matching allocates on every match attempt

`consume_tag` rebuilds grapheme vectors via `graphemes::init_tag(tag)` each call, and returns `String` for matched terminals. Many leaf parsers call `tag(...)` repeatedly, turning simple token checks into allocation-heavy paths.

Relevant code:
- `consume_tag` (`src/syntax/src/lib.rs`)
- `tag` combinator (`src/syntax/src/parser.rs`)
- leaf token macros calling `tag` heavily (`src/syntax/src/base.rs`)

**Impact:** avoidable allocation churn in the most frequently executed code.

**Fix:**
- Fast-path ASCII/single-grapheme tags without building grapheme vectors.
- Precompute static tag grapheme slices for fixed tokens.
- Return lightweight references/spans from `tag` and only allocate `String` at AST construction boundaries.

---

### 3) Token-building patterns perform repeated temporary allocations

Many rules parse into `Vec<(A,B)>`, map into new vectors, then merge via `Token::merge_tokens`. Examples include `skip_till_eol`, `terminal_token`, and identifier-related parsing.

Relevant code:
- `skip_till_eol` (`src/syntax/src/parser.rs`)
- `terminal_token` and grammar identifier assembly (`src/syntax/src/grammar.rs`)

**Impact:** high temporary allocation and copy overhead.

**Fix:**
- Introduce span-based parsing primitives (start/end cursor) and defer token materialization.
- Prefer `fold_many0`/`recognize` patterns to avoid intermediate vectors.
- Add specialized token constructors for common cases.

---

### 4) Backtracking strategy is manual and broad

`expression` and `factor` attempt many alternatives by cloning input and trying parsers sequentially. This multiplies work for near-miss alternatives.

Relevant code:
- `expression` and `factor` fallback chains (`src/syntax/src/expressions.rs`)

**Impact:** superlinear behavior on complex inputs and poor worst-case latency.

**Fix:**
- Convert to `alt(...)` with grammar refactoring to reduce ambiguity.
- Insert `cut(...)` once a discriminating prefix is consumed.
- Reorder alternatives by cheap discriminators / frequency.

---

### 5) Unicode handling does extra work on ASCII-heavy input

Parser stores input as grapheme vectors and repeatedly calls width/newline checks for each consumed grapheme, even in ASCII paths.

Relevant code:
- `graphemes::init_source`, `consume_one`, `consume_tag`, width checks (`src/syntax/src/lib.rs`)

**Impact:** additional CPU per character for common source files.

**Fix:**
- Add an ASCII fast path (byte-indexed scanner) and fallback to grapheme mode when non-ASCII is detected.
- Cache newline/width metadata per grapheme during initialization.

---

## Implementation Roadmap

### Phase 1: Low-risk wins (1-2 days)
1. Add criterion benchmarks for representative programs and pathological backtracking cases.
2. Remove per-match `init_tag` allocations for common fixed tokens.
3. Reorder `expression`/`factor` alternatives by cheapest discriminators.

### Phase 2: Structural improvements (3-5 days)
1. Refactor `ParseString` to cheap-clone cursor state + shared error log.
2. Replace manual fallback chains with `alt` and strategic `cut`.
3. Replace vector-heavy token assembly with span-based builders.

### Phase 3: Advanced path (optional, 1-2 weeks)
1. Introduce ASCII fast scanner with grapheme fallback.
2. Evaluate parser memoization (packrat-like cache) for worst-case ambiguous constructs.

## Measurement Plan

Track before/after for:
- Throughput (MB/s)
- Mean/95p parse latency per file
- Allocations/op and bytes/op (`heaptrack`, `dhat`, or jemalloc profiling)
- Backtracking counters (instrument parser alternative attempts)

Suggested benchmark sets:
- Real project corpus (`.mec` files)
- Deeply nested expressions
- Ambiguous-prefix stress tests
- Unicode-heavy documents

## Proposed Success Criteria

- 30-50% reduction in allocations for representative files.
- 20-40% lower p95 parse latency on expression-heavy inputs.
- Significant reduction in alternative-attempt counts for `expression`/`factor`.

## Notes

These recommendations are intentionally incremental: Phase 1 and 2 should yield meaningful gains without changing language semantics.
