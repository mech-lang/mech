# Concrete production consumer acceptance cells

`consumer-acceptance-cells.tsv` names one positive and one negative cell for each
of the frozen 27 consumer contracts: **54 cells**. This supplements, rather than
changes, `consumer-contracts.tsv`. Input and failure policies are copied from the
frozen contract rows; actual entry points, exact tests, explicit recipes and
observable oracles are recorded separately.

The ledger was prepared by source inspection at frozen C
`e31d08260fac40bcf7982e854cc5d2623a606347`. No cargo or browser executions were
started by this audit addendum. The 24 `recorded-pass` rows refer to named tests
verified present and passing in the recorded C runtime-library log, not a new
run. Five `partially-tested` rows preserve the exact remaining method/ordering
boundary. Nine `untested` rows have a named executable recipe but no exact-C
production result; 16 `blocked` rows name an existing G21/G22/G23 owner. A cell is
an acceptance obligation, not another inferred implementation defect.

Important reconciliations:

- `runtime.module-index` is also owned by G21. C's
  `src/runtime/src/runtime/module/mod.rs:44–49` still prefers a supplied Program
  tree over retained source and feeds that `SourceIndex` into resolved/record
  context materialization. A passing canonical transfer test with no tree does
  not exercise the conflicting-authority branch.
- C's `insert_string` now calls canonical strict admission; `with_string`
  intentionally retains malformed source and diagnostics without publishing
  semantic/index facts. That difference is the frozen infallible-builder
  contract, not two interchangeable authorities. Exact production-method tests
  exist for both and were found passing in the runtime log.
- C's `FileSourceResolver::resolve` uses the canonical revision/admission helper.
  Existing detailed identity tests call `resolve_canonical`, a separate method
  body; they are marked partial until repeated through the actual trait entry.
- C's browser bootstrap now returns original source-map text, and the actual
  REPL formatter uses the canonical renderer. Those changes are real. Actual
  `WasmDocument` construction still decodes Program and its interactive tree
  ignores candidate source; prepared renderer/session passes cannot qualify the
  real browser entry.
- Canonical bundle envelopes embed root source. A missing separate external root
  source is not automatically invalid if the standalone envelope is valid.
  Stale served text, invalid embedded source identity and missing/stale declared
  dependencies are the concrete negative admission cases.
- The stale readiness sentence that an explicit-dependency provider-count test
  “still fails in C” is not used as current failure evidence. The recorded exact
  C runtime run has two failures (factorial and comprehension initializer).
  Ordered transitive explicit roots have a separate demonstrated G16 witness.
- Activation-scope sends, unsupported compute regions and invalid configuration
  fields have existing target policies. This ledger does not ask for new
  product decisions to continue auditing them.

Commands containing an existing exact test are immediately runnable on the
stated feature profile. Recipes explicitly describing a new process/browser
assertion are unimplemented test work, never reported as passing. Each command
must execute at least one test. `--lib` unit tests, compiler probes and isolated
WASM compilation are never substituted for actual process/browser execution.
Full configured particle and EKF application requirements remain attached to
`cli.compute-inline`, `serve.workspace-render` and
`wasm.mixed-compute-source`; the frozen engine EKF efficacy fixture is separately
qualified and does not substitute for those application entry points.
