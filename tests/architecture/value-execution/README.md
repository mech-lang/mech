# Value execution legacy boundary

`legacy-boundary.json` is a deletion-friendly inventory of legacy value and
turn-coordination dependencies. Each approval names a repository-relative
production path, a stable containing scope, and the maximum number of literal
occurrences allowed in that scope. Approved occurrences may disappear without
updating the manifest; new paths, new scopes, and count growth fail the audit.

Run the audit with:

```sh
python scripts/check-value-execution-boundary.py
```

The scan covers Rust production sources beneath `src`, `machines`, and `hosts`.
Directories named `tests`, `benches`, `examples`, or `fixtures`, plus embedded
`*_tests` directories, are excluded. The checker tests document this treatment
with temporary source trees.
