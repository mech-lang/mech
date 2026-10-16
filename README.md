# Pending compiler and workload sources

This bundle preserves source candidates for compiler and benchmark maintainers. Every candidate is **UNAPPLIED / UNRUN**. No build, parser run, test execution, JIT/Object invocation, linked application or benchmark was performed to prepare this bundle. Prior source-review receipts do not supply execution qualification.

`manifest.json` lists the frozen patch set and dependency boundaries. `file-inventory.json` binds every retained file by SHA256 and byte count. Patches, Rust test sources, Mech sources and binary input/expected fixtures preserve their original bytes. Metadata is a neutral portable summary; private paths, conversational handoffs, logs and build products are excluded.

The retained patches overlap and were prepared against different pinned source compositions. They are review material, not one automatically applicable patch series. Preserve current implementation fixes and compose additive hunks; do not replace current production files with older snapshots. Phase directories identify the exact test source belonging to each patch and follow-up. Existing committed test files are included only when the candidate changes them; byte-identical committed files are listed as omitted dependencies.

- Ordinary Iteration supplies source syntax, artifact encoding and resident execution; native Iteration depends on it plus native Match and linked Match.
- Native I64 precedes F64, then checked casts and its borrow follow-up. Native control remains Bool/U64 until wider kinds are explicitly composed and qualified.
- Linked Match follows native Match; its separate slot follow-up changes the earlier Match test helper.
- Native matrix uses the final engine aggregate, constant-output overlay and GPU implementation. The earlier symbolic-template patch is superseded inside the engine aggregate and is not duplicated.
- Generic byte IO uses the provider patch followed by the source-order test overlay.
- Nbody includes eleven prepared tests with a deliberately unmet strict extra-argument contract. The six other idiomatic programs retain explicit compiler/provider gaps and 113 input/output contracts. Expected-output files are retained oracle fixtures, not candidate execution results.

Already committed scalar witness/adapter/codegen/linked baseline source is referenced by commit in `baseline-dependencies.json`. This preservation bundle does not claim full workload completion, native matrix/control integration, same-binary secure/trusted qualification or timing results.
