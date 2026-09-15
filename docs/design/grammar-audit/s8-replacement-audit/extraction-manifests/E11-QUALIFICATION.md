# E11 — Frozen qualification closure and exact-tree proof

E11 is committed as `75ae76bf7d5a28781c7c6f20ce2484ca8d6a80a4` on
`codex/syntax-s8e11-qualification`, based on E10 `68eeb8828`.
It copies exactly the seven remaining files from frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae`:

| Files | Frozen responsibility |
| --- | --- |
| `.github/workflows/ci-full.yml`, `.github/workflows/ci.yml` | Attach canonical runtime execution, static bootstrap and browser compute checks. |
| `docs/design/grammar-audit/s8-consumer-readiness.tsv`, `s8-removal-manifest.tsv` | Record routed consumer status separately from deletion qualification. |
| `docs/design/syntax-s8-execution-rehearsal.mec` | Frozen execution rehearsal and acceptance record. |
| `src/syntax/tests/source_parser_consumer_inventory.rs`, `support/cutover_contract.rs` | Preserve the frozen caller census while enforcing explicitly routed callers and preventing routing from being counted as deletion evidence. |

The production branch contains no extraction audit files. `e11-symbols.json`
records full base/frozen identities and per-file hashes. No wording, expectation,
production code or semantic behavior was changed during this final extraction.

## Full-tree identity

These commands completed successfully at the E11 head:

```sh
git diff --exit-code 662d29b79 HEAD
git rev-parse 'HEAD^{tree}' '662d29b79^{tree}'
git status --short
```

`git diff` produced no output and exited zero. Both complete Git tree identities
are **`fdc18343936a6123e2370275eb63d817b262746c`**. The worktree is clean.
This proves equality of all tracked files and modes, with no excluded paths,
rather than equality only of selected production directories.

The audit checker additionally reconstructs E11 from E10 and the seven exact
frozen files, rejects other tracked/untracked changes, and checks the tree identity:

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e11.py /private/tmp/mech-syntax-s8e11-qualification
```

The checker, whitespace check and Rust formatting checks passed. No Cargo test,
workflow dispatch, browser run, push or PR was performed by the extraction agent.
The matching tree proves the accumulated frozen implementation was split without
alteration; it does not turn frozen implementation failures or audit obligations
into passing acceptance results. Parent-owned validation and review remain separate.
