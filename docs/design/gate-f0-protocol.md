# Gate F0 evidence protocol

F0 qualifies one immutable v0.4 product subject. It does not change that
product.

## Frozen subject

- Commit: `6c0923339e4b1a7d47f9b109135059487e6919ae`
- Tree: `39125894b400e2960b6ecf188682d4289435e629`
- Protected product digest:
  `bd1f28cb3a9889754911ffa8d21f45551b0e0b865de2955e9aca31f5ed15ceba`

Any protected product change requires a separately reviewed correction, a new
base, and complete evidence regeneration. F0 has no exception mechanism.

## Recorded session

The controlled machine performs untimed build and cache preparation, followed
by exactly three recorded `B2 → D2 → D3` chains. All three chains must pass
their release-blocking gates.
Chain 1 supplies the canonical checked-in reports; chains 2 and 3 are retained
replications. A failed chain cannot be discarded, replaced, or rerun as part of
the same session.

Gate B uses exactly ten Criterion samples, ten persistent-NumPy samples, a
one-second warm-up, a three-second measurement window, and 4,096 turns per
sample. Gate D uses exactly ten samples of 4,096 turns for each declared lane.
The existing workload definitions, formulas, thresholds, cold-path controls,
and trajectory keys remain unchanged. Absolute and route-relative timing
comparisons remain visible as advisory findings. Gate B `executor_tax`,
`legacy_gap_closure`, `raw_epoch_ratio`, `tail_stability`,
`complete_turn_control_ratio`, and timing-only `source_bytecode_equivalence`;
Gate D2 `resident_raw_ratio`, `legacy_gap_closure`, `complete_d1_ratio`,
`kernel_d1_ratio`, and `source_bytecode_ratio`; and Gate D3
`d2_pure_regression` and `source_bytecode_ratio` do not block release
qualification. Semantic source/bytecode equivalence remains blocking through
the correctness hashes, exact trajectories, and exact effect results.
Correctness, zero allocation, fixed publication behavior, history/epoch
independence, effects, replay, and provenance remain blocking.

## Evidence and trust

Each report records the product commit/tree, reviewed protocol commit, evidence
generation commit, environment ID, session ID, chain ID, and workflow run
identity. The manifest records those identities, the three chains, exact report
hashes, final decisions, and selected/full CI run identities.

`scripts/f0_contract.py` is the single authority for formulas, thresholds,
provenance, raw reconstruction, replication, manifest validation, and closeout.
It reconstructs Gate B from retained Criterion samples, structural probes, and
persistent-NumPy output before recomputing every B2, D2, and D3 hard gate. D2
must authenticate its chain's exact B2 bytes. D3 must authenticate the exact
release-qualified D2 bytes before measurement begins.

The controlled environment pins only tools that affect these measurements:
the physical machine, macOS power and thermal conditions, Rust/Cargo, Python,
NumPy and its BLAS provider, `Cargo.lock`, compiler inputs, and thread controls.
NumPy is installed from an authenticated wheel into a fresh environment and its
installed files are verified against the wheel RECORD. Chrome, ChromeDriver,
Node, npm, and wasm-pack remain owned by their normal CI jobs.

Before the workflow exists on the default branch, the `f0-controlled` PR-label
route is authenticated from GitHub's event payload. The runner requires the
exact repository, label action, PR number, head repository, head branch, head
SHA, merge-workflow ref, and merge-workflow SHA. After registration on the
default branch, manual dispatch retains its exact branch/workflow SHA checks.

The session ledger is created before environment verification and retained on
success or failure. Untimed preparation may record commands and logs, but may
not produce reports or masquerade as a fourth evidence chain.

## Lifecycle

1. Review the protocol commit and immutable product-tree guard.
2. During protocol-only preparation, apply `ci:f0-focused` to run selected CI
   without the exhaustive matrix. The classifier accepts that deferral only
   while every changed path is in the registered F0 protocol and evidence
   surface; product or unrelated contract changes still force full CI.
3. Apply the `f0-controlled` label to dispatch
   `.github/workflows/f0-controlled.yml` on the exact reviewed PR head. Manual
   dispatch remains available after the workflow is registered on the default
   branch.
4. Commit the three retained chains and the evidence manifest, remove
   `ci:f0-focused`, and apply `ci:full`.
5. Run selected and full CI on that exact evidence head.
6. Record run IDs, attempts, conclusions, URLs, and the exact head in the
   closeout record or release tag.

The opening baseline Full CI run is
`https://github.com/mech-lang/mech/actions/runs/31929501386`. Its run ID,
attempt, head SHA, workflow, conclusion, and URL are supporting closeout facts;
committed GitHub API payloads are not treated as an independent trust root.

Before evidence exists, CI validates the protocol and frozen product tree. Once
the manifest contains evidence, CI also requires the value-system checker to
return zero findings and validates every retained chain. Selected and full CI
must both succeed on one exact head. Unknown manifest states or incomplete
evidence fail closed.
