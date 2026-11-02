# Value execution boundary

`legacy-boundary.json` retains the permanent shrink-only controls for deprecated
turn-coordination and pointer-identity mechanisms that are independent of the
removed universal value model. Each approval names a repository-relative
production path, a stable containing scope, and the maximum number of literal
occurrences allowed in that scope. Approved occurrences may disappear without
updating the manifest; new paths, new scopes, and count growth fail the audit.
The retired value types and bridge APIs themselves are globally prohibited by
`scripts/check-no-retired-value-system.py`.

Run the audit with:

```sh
python scripts/check-value-execution-boundary.py
```

The scan covers Rust production sources beneath `src`, `machines`, and `hosts`.
Directories named `tests`, `benches`, `examples`, or `fixtures`, plus embedded
`*_tests` directories, are excluded. The checker tests document this treatment
with temporary source trees.
