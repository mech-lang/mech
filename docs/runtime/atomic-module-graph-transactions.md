# Atomic module-graph transactions

Round 5 makes graph-producing runtime APIs atomic without replacing the
existing recursive `ModuleDependencyGraph` builder. Dependencies are still
resolved and compiled before their importers, import-edge ordering and
multiplicity are preserved, and the existing cycle and dependency errors
remain authoritative.

## Transaction-local publication

Each runtime execution transaction owns a private module journal. The journal
contains ordered module and module-version puts; it does not contain
activation changes. Transaction-aware graph construction reads its owning
journal first and then the committed store. Other transactions and ordinary
committed-store reads cannot see provisional records.

An operation savepoint records a journal mark alongside the existing staged
store, effect, capability, context, program, and live-state checkpoints.
Rolling back an operation truncates the module journal to that mark and
rebuilds its private indexes. Earlier successful work in an explicit
transaction therefore remains provisional when a later graph build fails.

At commit, module and module-version records are added to
`RuntimeStoreCommit`. The in-memory store validates and applies them with the
other staged categories through one clone, apply, and swap. Equal publication
is idempotent; a conflicting identity rejects the complete batch.

Module invariants are checked before transactional effects are prepared.
Owners, dependency versions, and import-edge targets must be visible in the
same journal or the committed store.

## Retained-root boundary

`resolve_and_run_root_module_with_context` enters
`with_atomic_program_operation` before resolving or compiling the root.
Source-resolution events, compiled records, dependency execution, retained
root installation, live registration, effects, capabilities, and ordinary
staged store work therefore share one transaction.

A pre-store failure rolls back the graph and retained program together. For an
implicit operation, the hidden transaction is removed. For an explicit
transaction, a failed operation restores its savepoint while preserving
earlier provisional work; a retryable commit failure keeps the complete outer
transaction available for retry or abort.

Once the durable store commit succeeds, graph and program rollback is no
longer permitted. A transactional participant commit failure is external
commit indeterminacy: the graph and retained program stay durable, all
prepared participants receive the commit decision, and the runtime is
poisoned. Failure to deliver an after-commit effect also leaves the graph and
program committed, but follows the existing healthy delivery-failure outcome.

Resolver reads are non-reversible observations. Round 5 stages the records and
events derived from those reads; it does not claim source snapshot consistency
or resolution conflict detection.

## Boundaries and exclusions

Standalone `run_module` remains isolated and does not install a retained
program. It can execute a provisional graph only when called with the owning
explicit transaction context.

Public `ensure_module` and module activation APIs retain their prior direct
store behavior and signatures. Activation is not staged and no activation
compare-and-set protocol is introduced.

Runtime-owned reactive steps and host-input turns use compact turn rollback
inside the execution transaction. They do not put module-journal or
whole-program checkpoint work in the reactive inner loop.

## Module-version identity

Module-version hashes use the `mech.module.version.full.v2` domain and include
the owning `ModuleId` before source and build inputs. Equal source text under
different canonical module identities therefore produces different valid
version IDs, while equal canonical identity and build inputs reuse one
version.

This design does not add module history, rewind records, or source snapshot
transactions. Resolver reads remain observations rather than isolated source
snapshots.
