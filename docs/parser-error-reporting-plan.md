# Parser Error Reporting Fix Plan (Mech ↔ Mechdown boundary)

## Problem statement

When a Mech parse fails inside normal code (for example, in a `match` arm), the parser currently drifts into Mechdown paragraph parsing and emits cascaded generic messages like `Unexpected paragraph element` and `NomErrorKind: Many1`.

Example:

```mech
x := [1 2 3 4]

y := x?
  | [h a ... t] -> a
  | * => 0.
```

Expected: a focused syntax error on line 4 that points to `->` and suggests `=>` for match arms.

Current: multiple paragraph-related errors on lines 3–5.

---

## Root-cause hypothesis

1. **`match_arm` requires `output_operator` (`=>`) but has no targeted diagnostic when another arrow is used**.
   - `match_arm` currently parses `guard_operator`, `pattern`, optional guard, then `output_operator`, then expression.
   - A wrong arrow token (`->`) causes a generic parser failure rather than a specific "expected `=>`" error.

2. **Error recovery skips too far and changes parse mode**.
   - `mech_code` recovers via `skip_till_end_of_statement`, which can leave parser state in a place where section parsing/paragraph parsing takes over.
   - This produces secondary errors unrelated to the first failure.

3. **Best-error selection prioritizes farthest cursor, not semantic quality**.
   - `alt_best` favors farthest progress; this can prefer low-quality downstream errors (e.g. paragraph-level) over the precise early syntax mismatch.

4. **Parser report is not deduplicated / ranked for primary cause**.
   - The unified error system is present, but parser emits a noisy set without promoting the root error.

---

## Implementation plan

## Phase 1 — Add explicit, high-quality match-arm arrow error

### 1.1 Add a dedicated parser for match output arrow
- Introduce a `match_output_operator` parser used only by `match_arm`.
- Behavior:
  - Accept `=>` (existing `output_operator`).
  - If input is `->` / `→`, emit a structured parse error:
    - Message: `Match arm expects '=>'. Found transition arrow '->'.`
    - Annotation range: the arrow token span.
    - Optional help text in message: `Use '=>' in match arms; '->' is for FSM transitions.`

### 1.2 Wire into `match_arm`
- Replace direct `output_operator(input)?` with a labeled call that guarantees this message is surfaced.
- Keep the current grammar unchanged (only diagnostics change).

### 1.3 Add focused tests
- Unit tests in expression parser module:
  - `| [h a ... t] -> a` produces one primary error at arrow location.
  - `| [h a ... t] => a` remains valid.

---

## Phase 2 — Tighten recovery boundaries in `mech_code`

### 2.1 Introduce context-aware recovery for mech statements
- Replace/augment `skip_till_end_of_statement` usage in `mech_code` with a boundary-aware skip routine:
  - stop on newline, semicolon, EOF,
  - **and** stop before section-level Mechdown sentinels (`subtitle`, prompt/list/quote markers, etc.) without consuming them.
- Goal: prevent handoff into paragraph parsing from a broken mech line.

### 2.2 Preserve original failing range as primary annotation
- When recovery creates `MechCode::Error`, keep the initial failure range (arrow span) as `cause_range`.
- Attach skipped span separately as context, not as the main blame range.

### 2.3 Add regression tests
- Parse the exact repro snippet and assert:
  - no `Unexpected paragraph element` message,
  - no `NomErrorKind: Many1` as primary report,
  - primary error line/column points to line 4 arrow.

---

## Phase 3 — Improve parser error ranking / consolidation

### 3.1 Rank parser errors by specificity before formatting
- During `parse()` / `parse_grammar()` report construction, rank errors with a simple heuristic:
  - explicit labeled/syntax messages (highest),
  - generic nom errors (lowest).
- Show top-ranked error first in formatter output.

### 3.2 Collapse redundant cascaded errors nearby
- If multiple errors occur on same line/adjacent columns after one hard failure, collapse into a single primary error + optional "suppressed N cascading errors" note.

### 3.3 Keep unified error API unchanged
- Perform ranking/collapse inside parser-side report building so callers do not need API changes.

---

## Proposed file touch points

- `src/syntax/src/expressions.rs`
  - add `match_output_operator` and targeted diagnostic path.

- `src/syntax/src/parser.rs`
  - tighten `mech_code` recovery boundaries.
  - keep root cause range during error placeholder generation.

- `src/syntax/src/lib.rs`
  - parser report ranking / de-dup helpers (if implemented at formatting stage).

- `src/syntax/src/mechdown.rs` (only if needed)
  - expose/centralize section boundary checks reused by recovery logic.

- `src/syntax` tests
  - add regression fixtures for wrong match arrow and mixed Mech/Mechdown contexts.

---

## Acceptance criteria

1. Given the repro snippet, the first and primary error is on **line 4** at `->` with message indicating `=>` is required.
2. No paragraph-level cascading diagnostics for this case.
3. Correct code using `=>` parses unchanged.
4. Existing FSM syntax using `->` / `→` remains unchanged and valid in FSM contexts.
5. Error report count for this scenario is reduced (ideally 1 primary error).

---

## Rollout order

1. Implement Phase 1 and tests (high impact, low risk).
2. Implement Phase 2 and regression tests (prevents mode drift).
3. Implement Phase 3 (quality polish / noise reduction).

This sequencing ensures immediate user-visible improvement after Phase 1 while preserving room for deeper recovery improvements.
